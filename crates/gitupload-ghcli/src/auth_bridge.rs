//! Zero-friction Dual Auth Bridge synchronizing credentials between `gh` CLI and OS Keyring.

use crate::lifecycle::GhLifecycleManager;
use gitupload_core::errors::{GhCliError, GhCliResult, GitUploadResult};
use gitupload_security::KeyringManager;
use secrecy::{ExposeSecret, SecretString};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Synchronizes authentication tokens bidirectionally between `gh` and Windows Credential Manager.
pub struct GhAuthBridge;

impl GhAuthBridge {
    /// Extracts the active token via `<gh_path> auth token` and stores it encrypted in `KeyringManager`.
    /// Returns the extracted token wrapped in `SecretString` so plaintext never touches logs or disk.
    pub async fn import_token_from_gh(
        gh_path: &Path,
        keyring: &KeyringManager,
        account_id: &str,
    ) -> GhCliResult<SecretString> {
        let mut cmd = tokio::process::Command::new(gh_path);
        cmd.args(["auth", "token"]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .await
            .map_err(|e| GhCliError::Process(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GhCliError::Auth(format!(
                "gh auth token returned error: {}",
                stderr.trim()
            )));
        }

        let token_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if token_raw.is_empty() {
            return Err(GhCliError::Auth(
                "gh auth token returned empty output".into(),
            ));
        }

        let secret = SecretString::from(token_raw);
        let vault_key = KeyringManager::github_account_key(account_id);
        keyring
            .store_token(&vault_key, &secret)
            .map_err(|e| GhCliError::Auth(format!("Failed to save token to Keyring: {e}")))?;

        Ok(secret)
    }

    /// Injects a PAT from OS Credential Vault into GitHub CLI via `gh auth login --with-token`.
    pub async fn export_token_to_gh(gh_path: &Path, token: &SecretString) -> GhCliResult<()> {
        let mut cmd = tokio::process::Command::new(gh_path);
        cmd.args(["auth", "login", "--with-token"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| GhCliError::Process(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(token.expose_secret().as_bytes())
                .await
                .map_err(|e| GhCliError::Io(e.to_string()))?;
            stdin
                .shutdown()
                .await
                .map_err(|e| GhCliError::Io(e.to_string()))?;
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| GhCliError::Process(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(GhCliError::Auth(format!(
                "gh auth login failed: {}",
                stderr.trim()
            )))
        }
    }

    /// High-level convenience bridge syncing active `gh` auth into `keyring` if `gh` is present.
    pub async fn sync_token_to_keyring(
        keyring: &KeyringManager,
        account_id: &str,
    ) -> GitUploadResult<bool> {
        let Some(gh_path) = GhLifecycleManager::detect_gh() else {
            return Ok(false);
        };
        match Self::import_token_from_gh(&gh_path, keyring, account_id).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}
