use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

use crate::error::ProbeError;
use crate::models::FfprobeOutput;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct FfprobeConfig {
    pub binary_path: PathBuf,
    pub timeout: Duration,
    pub extra_args: Vec<String>,
}

impl Default for FfprobeConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("ffprobe"),
            timeout: DEFAULT_TIMEOUT,
            extra_args: Vec::new(),
        }
    }
}

impl FfprobeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn binary_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.binary_path = path.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn extra_arg(mut self, arg: impl Into<String>) -> Self {
        self.extra_args.push(arg.into());
        self
    }
}

/// Executes `ffprobe` against the file at `path` using default configuration.
pub async fn probe(path: impl AsRef<Path>) -> Result<FfprobeOutput, ProbeError> {
    probe_with_config(path, &FfprobeConfig::default()).await
}

/// Executes `ffprobe` against the file at `path` using the given `config`.
pub async fn probe_with_config(
    path: impl AsRef<Path>,
    config: &FfprobeConfig,
) -> Result<FfprobeOutput, ProbeError> {
    let path = path.as_ref();

    let mut cmd = Command::new(&config.binary_path);
    cmd.args([
        "-v",
        "quiet",
        "-print_format",
        "json",
        "-show_format",
        "-show_streams",
        "-show_chapters",
    ]);

    for arg in &config.extra_args {
        cmd.arg(arg);
    }

    cmd.arg(path);

    let output_future = cmd.output();
    let output = match tokio::time::timeout(config.timeout, output_future).await {
        Ok(res) => res.map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                ProbeError::BinaryNotFound {
                    binary: config.binary_path.display().to_string(),
                    source: err,
                }
            } else {
                ProbeError::Io(err)
            }
        })?,
        Err(_) => return Err(ProbeError::Timeout { duration: config.timeout }),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ProbeError::ExecutionFailed {
            exit_code: output.status.code(),
            stderr,
        });
    }

    parse(&output.stdout)
}

/// Parses ffprobe JSON bytes into typed structures.
pub fn parse(json_bytes: &[u8]) -> Result<FfprobeOutput, ProbeError> {
    serde_json::from_slice(json_bytes).map_err(ProbeError::Deserialization)
}

/// Parses a ffprobe JSON string into typed structures.
pub fn parse_str(json_str: &str) -> Result<FfprobeOutput, ProbeError> {
    serde_json::from_str(json_str).map_err(ProbeError::Deserialization)
}
