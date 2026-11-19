#![forbid(unsafe_code)]
//! Standalone compressed `.bundle` cold-storage archive generator for tracked repositories.

use gitupload_core::errors::{GitError, GitResult};
use std::path::{Path, PathBuf};
use tracing::info;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates self-contained `git bundle` backup archives for disaster recovery.
pub struct GitBackupEngine;

impl GitBackupEngine {
    /// Generates `<repo_name>_<timestamp>.bundle` containing all refs in `backup_dir`.
    pub async fn create_repo_bundle(
        repo_path: &Path,
        backup_dir: &Path,
        repo_name: &str,
    ) -> GitResult<PathBuf> {
        tokio::fs::create_dir_all(backup_dir)
            .await
            .map_err(|e| GitError::Engine(format!("Backup dir creation failed: {e}")))?;

        let safe_name: String = repo_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let date_str = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let bundle_path = backup_dir.join(format!("{safe_name}_{date_str}.bundle"));

        let mut cmd = tokio::process::Command::new("git");
        cmd.current_dir(repo_path).args([
            "bundle",
            "create",
            &bundle_path.to_string_lossy(),
            "--all",
        ]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .await
            .map_err(|e| GitError::Engine(format!("git bundle spawn failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GitError::Engine(format!(
                "git bundle create failed: {}",
                stderr.trim()
            )));
        }

        info!("Generated Git backup bundle at {}", bundle_path.display());
        Ok(bundle_path)
    }
}
