//! Low-overhead debounced filesystem monitor built on `notify-debouncer-mini` + Tokio channels.

use gitupload_core::errors::{GitUploadError, GitUploadResult};
use notify::RecursiveMode;
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Debounced change notification emitted when tracked files change outside `.git/`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatcherAlert {
    /// Watched repository root path.
    pub repo_path: String,
    /// Relative paths modified within the debounce window.
    pub changed_paths: Vec<String>,
}

/// Watches repository directories and emits debounced `WatcherAlert` events.
pub struct RepoWatcher {
    watched_paths: Vec<PathBuf>,
    debounce_duration: Duration,
}

impl Default for RepoWatcher {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

impl RepoWatcher {
    /// Creates a new watcher with the specified debounce interval (default 500ms).
    pub fn new(debounce: Duration) -> Self {
        Self {
            watched_paths: Vec::new(),
            debounce_duration: debounce,
        }
    }

    /// Registers a directory path to be watched when `spawn` is called.
    pub fn watch_path(&mut self, path: &Path) -> GitUploadResult<()> {
        if !path.exists() {
            return Err(GitUploadError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Watch path does not exist: {}", path.display()),
            )));
        }
        let canonical = path.to_path_buf();
        if !self.watched_paths.contains(&canonical) {
            self.watched_paths.push(canonical);
        }
        Ok(())
    }

    /// Returns the currently registered watch paths.
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }

    /// Starts watching all registered directories on a dedicated thread, forwarding debounced events
    /// to `tx` until `cancel` is triggered.
    pub fn spawn(
        &self,
        tx: mpsc::Sender<WatcherAlert>,
        cancel: CancellationToken,
    ) -> GitUploadResult<()> {
        let paths = self.watched_paths.clone();
        let debounce = self.debounce_duration;

        std::thread::Builder::new()
            .name("gitupload-watcher".into())
            .spawn(move || {
                let (sync_tx, sync_rx) = std::sync::mpsc::channel();
                let Ok(mut debouncer) = new_debouncer(debounce, sync_tx) else {
                    return;
                };

                for dir in &paths {
                    let _ = debouncer.watcher().watch(dir, RecursiveMode::Recursive);
                }

                while !cancel.is_cancelled() {
                    match sync_rx.recv_timeout(Duration::from_millis(200)) {
                        Ok(Ok(events)) => {
                            let changed: Vec<String> = events
                                .into_iter()
                                .map(|ev| ev.path.to_string_lossy().replace('\\', "/"))
                                .filter(|p| !p.contains("/.git/") && !p.ends_with("/.git"))
                                .collect();
                            if !changed.is_empty() {
                                if let Some(root) = paths.first() {
                                    let _ = tx.blocking_send(WatcherAlert {
                                        repo_path: root.to_string_lossy().replace('\\', "/"),
                                        changed_paths: changed,
                                    });
                                }
                            }
                        }
                        Ok(Err(_)) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })
            .map_err(GitUploadError::Io)?;

        Ok(())
    }
}
