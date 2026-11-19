//! OS Credential Vault manager backed by Windows Credential Manager (DPAPI) with zeroized in-memory secrets.

use gitupload_core::errors::{GitUploadError, GitUploadResult};
use keyring::Entry;
use secrecy::{ExposeSecret, SecretString};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use zeroize::Zeroizing;

const SERVICE_NAME: &str = "FerritDesktop";
const LEGACY_SERVICE_NAME: &str = "GitUploadDesktop";

/// Trait allowing callers to pass either `&SecretString` or `&str` without exposing secrets outside the vault boundary.
pub trait SecretInput {
    /// Invokes `f` with a borrowed string slice of the secret value.
    fn with_secret<R>(&self, f: impl FnOnce(&str) -> R) -> R;
}

impl SecretInput for &str {
    fn with_secret<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        f(self)
    }
}

impl SecretInput for &String {
    fn with_secret<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        f(self.as_str())
    }
}

impl SecretInput for SecretString {
    fn with_secret<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        f(self.expose_secret())
    }
}

impl SecretInput for &SecretString {
    fn with_secret<R>(&self, f: impl FnOnce(&str) -> R) -> R {
        f(self.expose_secret())
    }
}

type InMemoryVaultStore = Arc<RwLock<HashMap<(String, String), Zeroizing<String>>>>;

#[derive(Clone, Debug)]
enum VaultBackend {
    OsCredentialManager,
    InMemory(InMemoryVaultStore),
}

/// Thread-safe credential manager storing PATs and Cloud AI API keys outside disk files and SQLite.
#[derive(Clone, Debug)]
pub struct KeyringManager {
    service: String,
    backend: VaultBackend,
}

impl Default for KeyringManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringManager {
    /// Creates a credential manager backed by the native OS Credential Vault (Windows Credential Manager).
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
            backend: VaultBackend::OsCredentialManager,
        }
    }

    /// Creates an isolated in-memory vault for deterministic offline CI and unit tests.
    pub fn new_in_memory() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
            backend: VaultBackend::InMemory(Arc::new(RwLock::new(HashMap::new()))),
        }
    }

    /// Creates an OS-backed credential manager scoped to a custom service namespace.
    pub fn with_service(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            backend: VaultBackend::OsCredentialManager,
        }
    }

    /// Builds the standardized GitHub account keyring key (`gitupload:github:account_<id>`).
    pub fn github_account_key(account_id: &str) -> String {
        format!("gitupload:github:account_{}", account_id.trim())
    }

    /// Builds the standardized Cloud AI provider keyring key (`gitupload:ai:provider_<name>`).
    pub fn ai_provider_key(provider: &str) -> String {
        format!("gitupload:ai:provider_{}", provider.trim().to_lowercase())
    }

    /// Encrypts and persists a sensitive token under `account` in the OS Credential Vault.
    pub fn store_token(&self, account: &str, token: impl SecretInput) -> GitUploadResult<()> {
        let trimmed_account = account.trim();
        if trimmed_account.is_empty() {
            return Err(GitUploadError::Vault(
                "Account identifier for keyring entry cannot be empty".into(),
            ));
        }

        token.with_secret(|raw| {
            if raw.trim().is_empty() {
                return Err(GitUploadError::Vault(
                    "Cannot store an empty token in the Credential Vault".into(),
                ));
            }

            match &self.backend {
                VaultBackend::OsCredentialManager => {
                    let entry = Entry::new(&self.service, trimmed_account)
                        .map_err(|e| GitUploadError::Vault(e.to_string()))?;
                    entry
                        .set_password(raw)
                        .map_err(|e| GitUploadError::Vault(e.to_string()))?;
                    Ok(())
                }
                VaultBackend::InMemory(map) => {
                    let mut guard = map
                        .write()
                        .map_err(|e| GitUploadError::Vault(format!("Vault lock poisoned: {e}")))?;
                    guard.insert(
                        (self.service.clone(), trimmed_account.to_string()),
                        Zeroizing::new(raw.to_string()),
                    );
                    Ok(())
                }
            }
        })
    }

    /// Retrieves a secret wrapped in `secrecy::SecretString` (zeroized automatically on drop).
    pub fn get_token(&self, account: &str) -> GitUploadResult<Option<SecretString>> {
        let trimmed_account = account.trim();
        if trimmed_account.is_empty() {
            return Ok(None);
        }

        match &self.backend {
            VaultBackend::OsCredentialManager => {
                let entry = Entry::new(&self.service, trimmed_account)
                    .map_err(|e| GitUploadError::Vault(e.to_string()))?;
                match entry.get_password() {
                    Ok(pass) => Ok(Some(SecretString::from(pass))),
                    Err(keyring::Error::NoEntry) => {
                        // Fallback check for legacy service name
                        if self.service == SERVICE_NAME {
                            if let Ok(legacy_entry) =
                                Entry::new(LEGACY_SERVICE_NAME, trimmed_account)
                            {
                                if let Ok(pass) = legacy_entry.get_password() {
                                    return Ok(Some(SecretString::from(pass)));
                                }
                            }
                        }
                        Ok(None)
                    }
                    Err(e) => Err(GitUploadError::Vault(e.to_string())),
                }
            }
            VaultBackend::InMemory(map) => {
                let guard = map
                    .read()
                    .map_err(|e| GitUploadError::Vault(format!("Vault lock poisoned: {e}")))?;
                Ok(guard
                    .get(&(self.service.clone(), trimmed_account.to_string()))
                    .map(|secret| SecretString::from(secret.as_str().to_string())))
            }
        }
    }

    /// Removes a stored credential from the OS Credential Vault.
    pub fn delete_token(&self, account: &str) -> GitUploadResult<()> {
        let trimmed_account = account.trim();
        if trimmed_account.is_empty() {
            return Ok(());
        }

        match &self.backend {
            VaultBackend::OsCredentialManager => {
                let entry = Entry::new(&self.service, trimmed_account)
                    .map_err(|e| GitUploadError::Vault(e.to_string()))?;
                match entry.delete_credential() {
                    Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                    Err(e) => Err(GitUploadError::Vault(e.to_string())),
                }
            }
            VaultBackend::InMemory(map) => {
                let mut guard = map
                    .write()
                    .map_err(|e| GitUploadError::Vault(format!("Vault lock poisoned: {e}")))?;
                guard.remove(&(self.service.clone(), trimmed_account.to_string()));
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyring() {
        let test_token = "ghp_1234567890abcdef1234567890abcdef12";
        let account = KeyringManager::github_account_key("unit-test-p1-ac03");

        // Verify in-memory vault semantics and SecretString redaction
        let mem_vault = KeyringManager::new_in_memory();
        mem_vault.store_token(&account, test_token).unwrap();
        let retrieved = mem_vault.get_token(&account).unwrap().unwrap();
        assert_eq!(retrieved.expose_secret(), test_token);
        assert!(!format!("{retrieved:?}").contains(test_token));
        mem_vault.delete_token(&account).unwrap();
        assert!(mem_vault.get_token(&account).unwrap().is_none());

        // Verify real OS Credential Vault (Windows Credential Manager) with clean teardown
        let os_vault = KeyringManager::new();
        os_vault
            .store_token(&account, SecretString::from(test_token.to_string()))
            .unwrap();
        let os_retrieved = os_vault.get_token(&account).unwrap().unwrap();
        assert_eq!(os_retrieved.expose_secret(), test_token);
        os_vault.delete_token(&account).unwrap();
        assert!(os_vault.get_token(&account).unwrap().is_none());
    }
}
