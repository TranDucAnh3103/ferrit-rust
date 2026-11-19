//! Status helper types and exports for `gitupload-ghcli`.

use crate::lifecycle::GhLifecycleManager;
use gitupload_core::models::GhCliStatus;

/// Alias for `GhCliStatus` status diagnostics.
pub type GhStatusReport = GhCliStatus;

impl GhLifecycleManager {
    /// Synchronous path-only diagnostic snapshot without spawning `gh --version`.
    pub fn quick_status(&self) -> GhCliStatus {
        match self.resolve_binary_path() {
            Some(path) => {
                let is_sandboxed = path.starts_with(self.sandbox_dir());
                GhCliStatus {
                    is_installed: true,
                    is_sandboxed,
                    version: None,
                    binary_path: Some(path.to_string_lossy().to_string()),
                }
            }
            None => GhCliStatus {
                is_installed: false,
                is_sandboxed: false,
                version: None,
                binary_path: None,
            },
        }
    }
}
