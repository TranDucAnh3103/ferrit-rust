#![forbid(unsafe_code)]
//! Cross-platform OS Credential Vault abstraction unified across Windows DPAPI, macOS Keychain, and Linux libsecret.

use crate::keyring::KeyringManager;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::SecretString;
use std::sync::Arc;

/// Unified cross-platform secret store delegating to native OS keyring backends
/// (Windows Credential Manager, Apple Keychain Services, or Linux D-Bus Secret Service).
#[derive(Clone, Debug)]
pub struct PlatformVault {
    inner: Arc<KeyringManager>,
}

impl PlatformVault {
    /// Connects to the active OS credential vault backend.
    pub fn system() -> Self {
        Self {
            inner: Arc::new(KeyringManager::new()),
        }
    }

    /// Creates an isolated ephemeral in-memory vault for automated cross-platform CI tests.
    pub fn ephemeral_in_memory() -> Self {
        Self {
            inner: Arc::new(KeyringManager::new_in_memory()),
        }
    }

    /// Returns the human-readable backend driver identifier for the host OS.
    pub fn active_backend_name() -> &'static str {
        #[cfg(target_os = "windows")]
        {
            "Windows Credential Manager (DPAPI)"
        }
        #[cfg(target_os = "macos")]
        {
            "Apple Keychain Services (com.gitupload.desktop)"
        }
        #[cfg(target_os = "linux")]
        {
            "freedesktop.org D-Bus Secret Service (libsecret)"
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            "Generic Keyring Vault"
        }
    }

    /// Stores `secret` under `account` inside the OS vault.
    pub async fn store(&self, account: &str, secret: &SecretString) -> GitUploadResult<()> {
        self.inner.store_token(account, secret)
    }

    /// Retrieves stored `SecretString` for `account`, returning `Err(Vault)` if absent.
    pub async fn retrieve(&self, account: &str) -> GitUploadResult<SecretString> {
        self.inner.get_token(account)?.ok_or_else(|| {
            GitUploadError::Vault(format!("Secret not found for account '{account}'"))
        })
    }

    /// Removes stored secret for `account` cleanly.
    pub async fn delete(&self, account: &str) -> GitUploadResult<()> {
        self.inner.delete_token(account)
    }
}

impl Default for PlatformVault {
    fn default() -> Self {
        Self::system()
    }
}
