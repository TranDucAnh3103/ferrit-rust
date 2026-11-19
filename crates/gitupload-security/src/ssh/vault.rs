//! DPAPI OS Credential Vault storage for SSH private key material (zero plaintext on disk).

use crate::keyring::KeyringManager;
use crate::ssh::keygen::GeneratedSshKeypair;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use zeroize::Zeroizing;

/// Safe chunk threshold well under Windows Credential Manager's 2560-byte blob limit.
const VAULT_CHUNK_SIZE: usize = 900;

/// Manages SSH private key lifecycle inside [`KeyringManager`] without touching disk or SQLite.
#[derive(Clone, Debug)]
pub struct SshVault {
    keyring: Arc<KeyringManager>,
}

impl SshVault {
    /// Creates an SSH vault backed by the native OS Credential Manager.
    pub fn new() -> Self {
        Self {
            keyring: Arc::new(KeyringManager::new()),
        }
    }

    /// Creates an SSH vault backed by a provided [`KeyringManager`] (e.g. `new_in_memory()` in tests).
    pub fn with_keyring(keyring: Arc<KeyringManager>) -> Self {
        Self { keyring }
    }

    fn base_account_key(key_id: &str) -> String {
        format!("ssh_key:{}", key_id.trim())
    }

    /// Persists a generated SSH private key payload into the OS Credential Vault.
    pub fn store_generated_keypair(&self, keypair: &GeneratedSshKeypair) -> GitUploadResult<()> {
        self.store_private_material(&keypair.descriptor.key_id, &keypair.private_material_json)
    }

    /// Persists raw serialized key material cleanly under `ssh_key:<key_id>`, chunking if > 900 bytes.
    pub fn store_private_material(&self, key_id: &str, secret_json: &str) -> GitUploadResult<()> {
        let key_id = key_id.trim();
        if key_id.is_empty() {
            return Err(GitUploadError::Config(
                "SSH key_id cannot be empty".to_string(),
            ));
        }

        self.delete_private_key(key_id)?;

        if secret_json.len() <= VAULT_CHUNK_SIZE {
            return self
                .keyring
                .store_token(&Self::base_account_key(key_id), secret_json);
        }

        let chunks: Vec<&str> = secret_json
            .as_bytes()
            .chunks(VAULT_CHUNK_SIZE)
            .map(|c| std::str::from_utf8(c).unwrap_or_default())
            .collect();
        self.keyring.store_token(
            &format!("{}:chunks", Self::base_account_key(key_id)),
            &chunks.len().to_string(),
        )?;
        for (idx, chunk) in chunks.iter().enumerate() {
            self.keyring.store_token(
                &format!("{}:part:{idx}", Self::base_account_key(key_id)),
                *chunk,
            )?;
        }
        Ok(())
    }

    /// Loads serialized private key material into a zeroizing [`SecretString`].
    pub fn load_private_key(&self, key_id: &str) -> GitUploadResult<Option<SecretString>> {
        let base = Self::base_account_key(key_id);
        if let Some(single) = self.keyring.get_token(&base)? {
            return Ok(Some(single));
        }

        let Some(chunks_meta) = self.keyring.get_token(&format!("{base}:chunks"))? else {
            return Ok(None);
        };
        let chunk_count: usize =
            chunks_meta.expose_secret().trim().parse().map_err(|_| {
                GitUploadError::Vault("Corrupted SSH vault chunk count".to_string())
            })?;

        let mut combined = Zeroizing::new(String::new());
        for idx in 0..chunk_count {
            let part = self
                .keyring
                .get_token(&format!("{base}:part:{idx}"))?
                .ok_or_else(|| {
                    GitUploadError::Vault(format!("Missing SSH vault chunk {idx} for {key_id}"))
                })?;
            combined.push_str(part.expose_secret());
        }
        Ok(Some(SecretString::from(combined.to_string())))
    }

    /// Removes stored SSH private key material (including multi-part chunks) from the vault.
    pub fn delete_private_key(&self, key_id: &str) -> GitUploadResult<()> {
        let base = Self::base_account_key(key_id);
        let _ = self.keyring.delete_token(&base);
        if let Some(meta) = self.keyring.get_token(&format!("{base}:chunks"))? {
            if let Ok(count) = meta.expose_secret().trim().parse::<usize>() {
                for idx in 0..count {
                    let _ = self.keyring.delete_token(&format!("{base}:part:{idx}"));
                }
            }
            let _ = self.keyring.delete_token(&format!("{base}:chunks"));
        }
        Ok(())
    }
}

impl Default for SshVault {
    fn default() -> Self {
        Self::new()
    }
}
