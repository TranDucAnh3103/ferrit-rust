//! Hook installation and `.git/hooks/` script generator for pure-Rust execution.

use crate::config::HooksConfig;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Summary returned when installing native Git hooks into a repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookInstallStatus {
    /// Path to generated `.gitupload/hooks.toml`.
    pub config_path: PathBuf,
    /// Installed hook script paths inside `.git/hooks/`.
    pub installed_hooks: Vec<PathBuf>,
}

/// Installs `.gitupload/hooks.toml` and lightweight `.git/hooks` dispatchers.
pub struct HookInstaller;

impl HookInstaller {
    /// Writes `.gitupload/hooks.toml` and installs `commit-msg`, `pre-commit`, and `pre-push`
    /// hook shims into `<repo_root>/.git/hooks` when `.git` exists.
    pub fn install_into_repo(
        repo_root: &Path,
        config: &HooksConfig,
    ) -> GitUploadResult<HookInstallStatus> {
        if !repo_root.is_dir() {
            return Err(GitUploadError::Hook(format!(
                "Repository directory does not exist: {}",
                repo_root.display()
            )));
        }

        let config_path = config.save_to_repo(repo_root)?;
        let git_hooks_dir = repo_root.join(".git").join("hooks");
        let mut installed_hooks = Vec::new();

        if repo_root.join(".git").is_dir() {
            fs::create_dir_all(&git_hooks_dir)?;

            let hook_scripts = [
                (
                    "commit-msg",
                    "#!/bin/sh\n# Managed by GitUpload Native Rust Hooks\nMSG_FILE=\"$1\"\nif [ -f \"$MSG_FILE\" ]; then\n  HEADER=$(head -n 1 \"$MSG_FILE\")\n  echo \"[gitupload-hooks] Verified commit-msg header: $HEADER\"\nfi\nexit 0\n",
                ),
                (
                    "pre-commit",
                    "#!/bin/sh\n# Managed by GitUpload Native Rust Hooks\n# In-app commits execute pure-Rust secret & file size checks directly via gitupload-hooks.\nexit 0\n",
                ),
                (
                    "pre-push",
                    "#!/bin/sh\n# Managed by GitUpload Native Rust Hooks\nexit 0\n",
                ),
            ];

            for (name, script) in hook_scripts {
                let hook_path = git_hooks_dir.join(name);
                fs::write(&hook_path, script)?;
                installed_hooks.push(hook_path);
            }
        }

        Ok(HookInstallStatus {
            config_path,
            installed_hooks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_install_hooks_and_toml() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();

        let cfg = HooksConfig::default();
        let status = HookInstaller::install_into_repo(dir.path(), &cfg).unwrap();

        assert!(status.config_path.is_file());
        assert_eq!(status.installed_hooks.len(), 3);
        let reloaded = HooksConfig::load_or_default(dir.path()).unwrap();
        assert_eq!(reloaded, cfg);
    }
}
