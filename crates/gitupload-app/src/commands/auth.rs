//! Authentication and OS Credential Vault commands (`auth_*`).

use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_ghcli::GhAuthBridge;
use gitupload_security::KeyringManager;
use secrecy::SecretString;
use serde::Serialize;
use tauri::State;

/// Vault status metadata returned to the UI without exposing raw secrets.
#[derive(Debug, Clone, Serialize)]
pub struct AuthAccountSummary {
    /// Account identifier.
    pub account_id: String,
    /// True if a GitHub token is currently stored in the OS Keyring.
    pub has_github_token: bool,
    /// Active provider keys stored in Keyring (`gemini`, `openai`, `claude`, `groq`, `openrouter`).
    pub configured_ai_providers: Vec<String>,
}

/// Saves a GitHub Personal Access Token directly into Windows Credential Manager.
#[tauri::command]
pub async fn auth_save_token(
    state: State<'_, AppState>,
    account_id: String,
    token: String,
) -> Result<(), IpcErrorPayload> {
    let secret = SecretString::from(token);
    let key = KeyringManager::github_account_key(&account_id);
    state
        .keyring
        .store_token(&key, &secret)
        .map_err(IpcErrorPayload::from)
}

/// Saves a Cloud AI API key for `provider` directly into Windows Credential Manager.
#[tauri::command]
pub async fn auth_save_ai_key(
    state: State<'_, AppState>,
    provider: String,
    api_key: String,
) -> Result<(), IpcErrorPayload> {
    let secret = SecretString::from(api_key);
    let key = KeyringManager::ai_provider_key(&provider);
    state
        .keyring
        .store_token(&key, &secret)
        .map_err(IpcErrorPayload::from)
}

/// Checks credential vault presence without returning plaintext tokens.
#[tauri::command]
pub async fn auth_list_accounts(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<AuthAccountSummary, IpcErrorPayload> {
    let id = account_id.unwrap_or_else(|| "default".to_string());
    let has_github_token = state
        .get_github_token(&id)
        .map_err(IpcErrorPayload::from)?
        .is_some();

    let mut configured_ai_providers = Vec::new();
    for provider in [
        "gemini",
        "openai",
        "claude",
        "groq",
        "openrouter",
        "default",
    ] {
        if state
            .get_ai_key(provider)
            .map_err(IpcErrorPayload::from)?
            .is_some()
        {
            configured_ai_providers.push(provider.to_string());
        }
    }

    Ok(AuthAccountSummary {
        account_id: id,
        has_github_token,
        configured_ai_providers,
    })
}

/// Dual Auth Bridge: extracts active GitHub token from `gh auth token` directly into Windows Credential Manager.
#[tauri::command]
pub async fn auth_sync_from_gh(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<bool, IpcErrorPayload> {
    let id = account_id.unwrap_or_else(|| "default".to_string());
    let gh_path = state
        .gh_cli
        .resolve_binary_path()
        .ok_or_else(|| GitUploadError::GhCli("GitHub CLI (gh) is not installed".into()))?;

    GhAuthBridge::import_token_from_gh(&gh_path, &state.keyring, &id)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    Ok(true)
}
