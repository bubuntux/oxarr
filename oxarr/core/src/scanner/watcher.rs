use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use tokio::sync::mpsc;

use super::is_media_file;

/// Real-time filesystem watcher for a library root path using debounced OS events.
pub struct LibraryWatcher {
    _debouncer: Debouncer<notify::RecommendedWatcher>,
    rx: mpsc::Receiver<Vec<PathBuf>>,
}

impl LibraryWatcher {
    /// Spawns a debounced watcher on `watch_path` with a debounce quiet window (e.g. 2 seconds).
    pub fn watch(watch_path: impl AsRef<Path>, debounce_timeout: Duration) -> Result<Self> {
        let watch_path = watch_path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel(64);

        let mut debouncer = new_debouncer(
            debounce_timeout,
            move |res: Result<Vec<DebouncedEvent>, _>| match res {
                Ok(events) => {
                    let media_paths: Vec<PathBuf> = events
                        .into_iter()
                        .map(|e| e.path)
                        .filter(|p| is_media_file(p))
                        .collect();

                    if !media_paths.is_empty() {
                        let _ = tx.blocking_send(media_paths);
                    }
                }
                Err(err) => {
                    tracing::error!(error = ?err, "debounced watcher error");
                }
            },
        )
        .context("failed to initialize debounced filesystem watcher")?;

        debouncer
            .watcher()
            .watch(&watch_path, notify::RecursiveMode::Recursive)
            .with_context(|| format!("failed to watch directory {}", watch_path.display()))?;

        Ok(Self {
            _debouncer: debouncer,
            rx,
        })
    }

    /// Asynchronously waits for the next batch of debounced media file events.
    pub async fn next_events(&mut self) -> Option<Vec<PathBuf>> {
        self.rx.recv().await
    }
}
