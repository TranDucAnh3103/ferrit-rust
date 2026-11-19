//! Declarative native Git hooks configuration (`.gitupload/hooks.toml` / `.githooks.toml`).

use gitupload_core::errors::{GitUploadError, GitUploadResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Top-level native hooks configuration stored per repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HooksConfig {
    /// Configuration for `commit-msg` hook.
    pub commit_msg: CommitMsgConfig,
    /// Configuration for `pre-commit` hook.
    pub pre_commit: PreCommitConfig,
    /// Configuration for `pre-push` hook.
    pub pre_push: PrePushConfig,
}

/// Rules enforced by the `commit-msg` hook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitMsgConfig {
    /// Whether `commit-msg` validation is active.
    pub enabled: bool,
    /// Require Conventional Commits 1.0.0 format (`<type>(<scope>): <description>`).
    pub enforce_conventional_commits: bool,
    /// Maximum allowed characters in the commit header line.
    pub max_header_length: usize,
}

/// Rules enforced by the `pre-commit` hook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreCommitConfig {
    /// Whether `pre-commit` scanning is active.
    pub enabled: bool,
    /// Reject commits containing API tokens, PATs, or private keys.
    pub block_secrets: bool,
    /// Maximum file size in MiB before recommending Git LFS.
    pub max_file_size_mb: usize,
    /// Reject staging unencrypted `.env` files.
    pub block_env_files: bool,
}

/// Rules enforced by the `pre-push` hook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrePushConfig {
    /// Whether `pre-push` protection is active.
    pub enabled: bool,
    /// Branches protected from direct pushes without a Pull Request.
    pub prevent_direct_push_branches: Vec<String>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            commit_msg: CommitMsgConfig {
                enabled: true,
                enforce_conventional_commits: true,
                max_header_length: 72,
            },
            pre_commit: PreCommitConfig {
                enabled: true,
                block_secrets: true,
                max_file_size_mb: 50,
                block_env_files: true,
            },
            pre_push: PrePushConfig {
                enabled: true,
                prevent_direct_push_branches: vec!["main".into(), "master".into()],
            },
        }
    }
}

impl HooksConfig {
    /// Canonical config path inside a repository (`.gitupload/hooks.toml`).
    pub fn config_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".gitupload").join("hooks.toml")
    }

    /// Loads `.gitupload/hooks.toml` (or `.githooks.toml`), falling back to defaults if absent.
    pub fn load_or_default(repo_root: &Path) -> GitUploadResult<Self> {
        let primary = Self::config_path(repo_root);
        let fallback = repo_root.join(".githooks.toml");
        let target = if primary.is_file() {
            Some(primary)
        } else if fallback.is_file() {
            Some(fallback)
        } else {
            None
        };

        if let Some(path) = target {
            let content = fs::read_to_string(&path)?;
            let parsed: Self = toml::from_str(&content)
                .map_err(|e| GitUploadError::Hook(format!("Invalid hooks TOML config: {e}")))?;
            Ok(parsed)
        } else {
            Ok(Self::default())
        }
    }

    /// Persists hooks configuration as TOML into `.gitupload/hooks.toml`.
    pub fn save_to_repo(&self, repo_root: &Path) -> GitUploadResult<PathBuf> {
        let path = Self::config_path(repo_root);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)
            .map_err(|e| GitUploadError::Hook(format!("Failed to serialize hooks config: {e}")))?;
        fs::write(&path, body)?;
        Ok(path)
    }
}
