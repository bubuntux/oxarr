pub mod watcher;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use oxarr_parser::ParsedMediaInfo;
use oxarr_probe::{FfprobeOutput, ProbeError};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Semaphore};
use walkdir::WalkDir;

/// Standard video media file extensions recognized by the scanner.
pub const MEDIA_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts",
];

/// Result of scanning and locally inspecting a media file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedMediaFile {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub hash_xxh3: Option<String>,
    pub parsed: ParsedMediaInfo,
    pub probe: Option<FfprobeOutput>,
}

/// Checks if a file path has a supported video extension.
pub fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_lowercase();
            MEDIA_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// Discovers media files under `root_dir` by walking the directory tree.
pub fn discover_media_files(root_dir: impl AsRef<Path>) -> Vec<PathBuf> {
    WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_media_file(path))
        .collect()
}

/// Computes a 64-bit XXH3 hex checksum of a file by streaming in 64 KiB chunks.
pub async fn compute_xxh3(path: impl AsRef<Path>) -> Result<String> {
    use tokio::io::AsyncReadExt;
    let path = path.as_ref();
    let mut file = tokio::fs::File::open(path)
        .await
        .with_context(|| format!("failed to open file for hashing {}", path.display()))?;
    let mut hasher = xxhash_rust::xxh3::Xxh3::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:016x}", hasher.digest()))
}

/// Inspects a single media file: parses filename tokens, computes XXH3 hash, and probes streams via ffprobe.
pub async fn inspect_file(path: impl AsRef<Path>) -> Result<ScannedMediaFile> {
    let path = path.as_ref().to_path_buf();
    let metadata = tokio::fs::metadata(&path)
        .await
        .with_context(|| format!("failed to read file metadata for {}", path.display()))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    let parsed = oxarr_parser::parse(file_name);
    let hash_xxh3 = match compute_xxh3(&path).await {
        Ok(hash) => Some(hash),
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "failed to compute xxh3 hash");
            None
        }
    };
    let probe = match oxarr_probe::probe(&path).await {
        Ok(output) => Some(output),
        Err(ProbeError::ExecutionFailed { ref stderr, .. }) => {
            tracing::warn!(path = %path.display(), error = %stderr, "ffprobe warning on inspected file");
            None
        }
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "ffprobe failed on inspected file");
            None
        }
    };

    Ok(ScannedMediaFile {
        path,
        size_bytes: metadata.len(),
        hash_xxh3,
        parsed,
        probe,
    })
}

/// Concurrent library scanner with bounded worker parallelism.
pub struct LibraryScanner {
    concurrency_limit: usize,
}

impl Default for LibraryScanner {
    fn default() -> Self {
        Self {
            concurrency_limit: 4,
        }
    }
}

impl LibraryScanner {
    pub fn new(concurrency_limit: usize) -> Self {
        Self {
            concurrency_limit: concurrency_limit.max(1),
        }
    }

    /// Scans a directory and streams inspected media files over an mpsc channel.
    pub fn scan_directory(&self, root_dir: impl AsRef<Path>) -> mpsc::Receiver<ScannedMediaFile> {
        let files = discover_media_files(root_dir);
        let (tx, rx) = mpsc::channel(self.concurrency_limit * 2);
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));

        tokio::spawn(async move {
            for file_path in files {
                let permit = match semaphore.clone().acquire_owned().await {
                    Ok(permit) => permit,
                    Err(_) => break,
                };
                let tx = tx.clone();

                tokio::spawn(async move {
                    let _permit = permit;
                    match inspect_file(&file_path).await {
                        Ok(scanned) => {
                            let _ = tx.send(scanned).await;
                        }
                        Err(err) => {
                            tracing::error!(path = %file_path.display(), error = %err, "failed to inspect file");
                        }
                    }
                });
            }
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_media_files() {
        assert!(is_media_file(Path::new("movie.mkv")));
        assert!(is_media_file(Path::new("movie.MP4")));
        assert!(!is_media_file(Path::new("movie.nfo")));
        assert!(!is_media_file(Path::new("sample.txt")));
    }

    #[tokio::test]
    async fn computes_xxh3_checksum() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let mut file = NamedTempFile::new().expect("temp file");
        file.write_all(b"oxarr media test content").expect("write test content");
        file.flush().expect("flush");

        let hash = compute_xxh3(file.path()).await.expect("hash computed");
        assert_eq!(hash.len(), 16); // 64-bit hex
    }
}
