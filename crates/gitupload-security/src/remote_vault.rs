//! Per-remote OS Credential Vault lookup and storage for HTTPS/Git hosting endpoints.

use crate::keyring::{KeyringManager, SecretInput};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::SecretString;
use std::sync::Arc;
use url::Url;

/// Resolves and stores per-remote authentication tokens securely inside the OS Credential Vault.
#[derive(Clone, Debug)]
pub struct RemoteVaultResolver {
    keyring: Arc<KeyringManager>,
}

impl RemoteVaultResolver {
    /// Creates a resolver backed by the OS Credential Vault (`Windows Credential Manager`).
    pub fn new() -> Self {
        Self {
            keyring: Arc::new(KeyringManager::new()),
        }
    }

    /// Creates a resolver wrapping an explicit [`KeyringManager`] (useful for isolated in-memory tests).
    pub fn with_keyring(keyring: Arc<KeyringManager>) -> Self {
        Self { keyring }
    }

    /// Extracts the normalized hostname from an HTTPS, SSH, or SCP-style (`git@host:owner/repo.git`) remote URL.
    pub fn extract_host(remote_url: &str) -> GitUploadResult<String> {
        let trimmed = remote_url.trim();
        if trimmed.is_empty() {
            return Err(GitUploadError::Config(
                "Remote URL cannot be empty when resolving credentials".to_string(),
            ));
        }

        if let Ok(parsed) = Url::parse(trimmed) {
            if let Some(host) = parsed.host_str() {
                return Ok(host.to_ascii_lowercase());
            }
            if parsed.scheme().eq_ignore_ascii_case("file") {
                return Ok("localhost".to_string());
            }
        }

        // Handle SCP-style SSH remotes like `git@github.com:org/repo.git`
        if let Some((_, rhs)) = trimmed.split_once('@') {
            if let Some((host, _)) = rhs.split_once(':') {
                let host_clean = host.trim().to_ascii_lowercase();
                if !host_clean.is_empty() {
                    return Ok(host_clean);
                }
            }
        }

        // Local filesystem remote paths do not require external host credentials
        if trimmed.starts_with('/')
            || trimmed.starts_with('.')
            || trimmed.contains('\\')
            || (trimmed.len() >= 2 && trimmed.as_bytes()[1] == b':')
        {
            return Ok("localhost".to_string());
        }

        Err(GitUploadError::Config(format!(
            "Unable to extract host from remote URL: {trimmed}"
        )))
    }

    /// Computes the standardized Credential Manager account identifier `remote:<host>:<auth_account>`.
    pub fn vault_key(remote_url: &str, auth_account: &str) -> GitUploadResult<String> {
        let host = Self::extract_host(remote_url)?;
        let account = auth_account.trim();
        if account.is_empty() {
            return Err(GitUploadError::Config(
                "Remote auth_account cannot be empty".to_string(),
            ));
        }
        Ok(format!("remote:{host}:{account}"))
    }

    /// Stores a PAT / OAuth credential for a specific remote endpoint inside the OS Credential Vault.
    pub fn store_remote_token(
        &self,
        remote_url: &str,
        auth_account: &str,
        secret: impl SecretInput,
    ) -> GitUploadResult<()> {
        let key = Self::vault_key(remote_url, auth_account)?;
        self.keyring.store_token(&key, secret)
    }

    /// Retrieves a stored PAT / OAuth credential for the given remote endpoint and account alias.
    pub fn resolve_remote_token(
        &self,
        remote_url: &str,
        auth_account: &str,
    ) -> GitUploadResult<Option<SecretString>> {
        let key = Self::vault_key(remote_url, auth_account)?;
        self.keyring.get_token(&key)
    }

    /// Deletes a stored remote credential from the OS Credential Vault.
    pub fn delete_remote_token(&self, remote_url: &str, auth_account: &str) -> GitUploadResult<()> {
        let key = Self::vault_key(remote_url, auth_account)?;
        self.keyring.delete_token(&key)
    }
}

impl Default for RemoteVaultResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn test_remote_vault_resolution() {
        let keyring = Arc::new(KeyringManager::new_in_memory());
        let resolver = RemoteVaultResolver::with_keyring(keyring);

        let gh_url = "https://github.com/acme/repo.git";
        let gl_url = "https://gitlab.company.internal:8443/team/project.git";

        resolver
            .store_remote_token(gh_url, "stephen-work", "ghp_mocktoken1234567890")
            .unwrap();
        resolver
            .store_remote_token(gl_url, "stephen-gitlab", "glpat_abcdef0987654321")
            .unwrap();

        let gh_token = resolver
            .resolve_remote_token("https://github.com/acme/other-repo.git", "stephen-work")
            .unwrap()
            .unwrap();
        assert_eq!(gh_token.expose_secret(), "ghp_mocktoken1234567890");

        let gl_token = resolver
            .resolve_remote_token(gl_url, "stephen-gitlab")
            .unwrap()
            .unwrap();
        assert_eq!(gl_token.expose_secret(), "glpat_abcdef0987654321");

        assert!(resolver
            .resolve_remote_token("https://bitbucket.org/org/repo.git", "stephen-work")
            .unwrap()
            .is_none());

        assert_eq!(
            RemoteVaultResolver::extract_host("git@gitea.local:devops/core.git").unwrap(),
            "gitea.local"
        );

        resolver
            .delete_remote_token(gh_url, "stephen-work")
            .unwrap();
        assert!(resolver
            .resolve_remote_token(gh_url, "stephen-work")
            .unwrap()
            .is_none());
    }
}
