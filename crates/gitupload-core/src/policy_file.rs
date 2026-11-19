//! Cross-platform file-based enterprise governance policy parser (`/etc/gitupload/policy.json` / ProgramData).

use crate::errors::{GitUploadError, GitUploadResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Enterprise governance settings enforced via IT-managed configuration files or Windows GPO.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterpriseSection {
    /// Display name of the governing organization.
    #[serde(default)]
    pub organization_name: String,
    /// Locks application into strict Air-Gap offline/intranet mode.
    #[serde(default)]
    pub strict_airgap: bool,
    /// Disables Cloud AI external endpoints globally.
    #[serde(default)]
    pub disable_cloud_ai: bool,
    /// Enforces zero telemetry emission.
    #[serde(default = "default_true")]
    pub disable_telemetry: bool,
    /// Optional allow-list of permitted Git remote URL prefixes / wildcards.
    #[serde(default)]
    pub allowed_remotes: Vec<String>,
    /// Requires GPG/SSH commit signing for all outgoing commits.
    #[serde(default)]
    pub enforced_commit_sign: bool,
    /// Optional corporate proxy endpoint override.
    #[serde(default)]
    pub corporate_proxy_url: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for EnterpriseSection {
    fn default() -> Self {
        Self {
            organization_name: String::new(),
            strict_airgap: false,
            disable_cloud_ai: false,
            disable_telemetry: true,
            allowed_remotes: Vec::new(),
            enforced_commit_sign: false,
            corporate_proxy_url: None,
        }
    }
}

/// Root JSON structure for `/etc/gitupload/policy.json` and `%ProgramData%\GitUpload\policy.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnterprisePolicyFile {
    /// Enterprise governance configuration block.
    #[serde(default)]
    pub enterprise: EnterpriseSection,
}

impl EnterprisePolicyFile {
    /// Returns the OS-standard system-wide policy path (`/etc/gitupload/policy.json` on Unix,
    /// `%ProgramData%\GitUpload\policy.json` on Windows).
    pub fn default_system_path() -> PathBuf {
        #[cfg(windows)]
        {
            let program_data =
                std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".to_string());
            PathBuf::from(program_data)
                .join("GitUpload")
                .join("policy.json")
        }
        #[cfg(not(windows))]
        {
            PathBuf::from("/etc/gitupload/policy.json")
        }
    }

    /// Loads an enterprise policy JSON file from `path` if present on disk.
    pub fn load_from_path(path: &Path) -> GitUploadResult<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let raw = std::fs::read_to_string(path).map_err(|e| {
            GitUploadError::Config(format!(
                "Failed reading enterprise policy '{}': {e}",
                path.display()
            ))
        })?;
        let parsed: Self = serde_json::from_str(&raw).map_err(|e| {
            GitUploadError::Config(format!(
                "Malformed enterprise policy JSON '{}': {e}",
                path.display()
            ))
        })?;
        Ok(Some(parsed))
    }

    /// Evaluates whether a Git remote `url` matches configured `allowed_remotes` rules.
    /// An empty `allowed_remotes` list allows any remote permitted by `AirGapGuard`.
    pub fn is_remote_allowed(&self, url: &str) -> bool {
        if self.enterprise.allowed_remotes.is_empty() {
            return true;
        }
        let target = url.trim().to_ascii_lowercase();
        self.enterprise.allowed_remotes.iter().any(|pattern| {
            let pat = pattern.trim().to_ascii_lowercase();
            if let Some(prefix) = pat.strip_suffix('*') {
                target.starts_with(prefix)
            } else {
                target == pat || target.contains(&pat)
            }
        })
    }
}
