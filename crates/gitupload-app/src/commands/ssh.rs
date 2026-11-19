//! Native SSH keypair generation, DPAPI Vault storage, Windows Named Pipe Agent control, and GitHub upload (`ssh_*`).

use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{SshKeyAlgorithm, SshKeyDescriptor};
use gitupload_security::generate_ssh_keypair;
use gitupload_storage::SshKeyRepository;
use serde::Serialize;
use tauri::State;

/// Status snapshot of the in-process pure-Rust SSH Agent Named Pipe daemon.
#[derive(Debug, Clone, Serialize)]
pub struct SshAgentStatusPayload {
    /// True when the Windows Named Pipe listener is active.
    pub is_running: bool,
    /// Active Named Pipe path (`\\.\pipe\openssh-ssh-agent`).
    pub pipe_name: String,
    /// Number of SSH identities currently unlocked in volatile memory.
    pub loaded_key_count: usize,
}

/// Lists all managed SSH key descriptors (public metadata only).
#[tauri::command]
pub async fn ssh_list_keys(
    state: State<'_, AppState>,
) -> Result<Vec<SshKeyDescriptor>, IpcErrorPayload> {
    SshKeyRepository::new(&state.storage)
        .list_keys()
        .map_err(IpcErrorPayload::from)
}

/// Generates a new Ed25519 or RSA 4096 SSH keypair, storing the private key inside Windows Credential Manager.
#[tauri::command]
pub async fn ssh_generate_key(
    state: State<'_, AppState>,
    title: String,
    algorithm: SshKeyAlgorithm,
) -> Result<SshKeyDescriptor, IpcErrorPayload> {
    let generated = tokio::task::spawn_blocking(move || generate_ssh_keypair(title, algorithm))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)?;

    state
        .ssh_vault
        .store_generated_keypair(&generated)
        .map_err(IpcErrorPayload::from)?;

    let repo = SshKeyRepository::new(&state.storage);
    repo.upsert_key(&generated.descriptor)
        .map_err(IpcErrorPayload::from)?;

    if state.ssh_agent.is_running() {
        let all_keys = repo.list_keys().map_err(IpcErrorPayload::from)?;
        let _ = state.ssh_agent.reload_identities(&all_keys);
    }

    Ok(generated.descriptor.clone())
}

/// Deletes an SSH keypair from both SQLite metadata and OS Credential Vault.
#[tauri::command]
pub async fn ssh_delete_key(
    state: State<'_, AppState>,
    key_id: String,
) -> Result<(), IpcErrorPayload> {
    state
        .ssh_vault
        .delete_private_key(&key_id)
        .map_err(IpcErrorPayload::from)?;

    let repo = SshKeyRepository::new(&state.storage);
    repo.delete_key(&key_id).map_err(IpcErrorPayload::from)?;

    if state.ssh_agent.is_running() {
        let remaining = repo.list_keys().map_err(IpcErrorPayload::from)?;
        let _ = state.ssh_agent.reload_identities(&remaining);
    }

    Ok(())
}

/// Returns the live status of the native SSH Agent Named Pipe daemon.
#[tauri::command]
pub async fn ssh_agent_status(
    state: State<'_, AppState>,
) -> Result<SshAgentStatusPayload, IpcErrorPayload> {
    Ok(SshAgentStatusPayload {
        is_running: state.ssh_agent.is_running(),
        pipe_name: state.ssh_agent.pipe_name().to_string(),
        loaded_key_count: state.ssh_agent.loaded_key_count(),
    })
}

/// Unlocks stored keys into volatile memory and starts the Windows Named Pipe SSH Agent daemon.
#[tauri::command]
pub async fn ssh_agent_start(
    state: State<'_, AppState>,
) -> Result<SshAgentStatusPayload, IpcErrorPayload> {
    let keys = SshKeyRepository::new(&state.storage)
        .list_keys()
        .map_err(IpcErrorPayload::from)?;

    state
        .ssh_agent
        .start(&keys)
        .await
        .map_err(IpcErrorPayload::from)?;

    Ok(SshAgentStatusPayload {
        is_running: state.ssh_agent.is_running(),
        pipe_name: state.ssh_agent.pipe_name().to_string(),
        loaded_key_count: state.ssh_agent.loaded_key_count(),
    })
}

/// Stops the Windows Named Pipe SSH Agent daemon and wipes keys from volatile memory.
#[tauri::command]
pub async fn ssh_agent_stop(
    state: State<'_, AppState>,
) -> Result<SshAgentStatusPayload, IpcErrorPayload> {
    state.ssh_agent.stop().await;
    Ok(SshAgentStatusPayload {
        is_running: state.ssh_agent.is_running(),
        pipe_name: state.ssh_agent.pipe_name().to_string(),
        loaded_key_count: state.ssh_agent.loaded_key_count(),
    })
}

/// Registers an existing managed SSH public key on the authenticated GitHub account (`POST /user/keys`).
#[tauri::command]
pub async fn ssh_upload_key_to_github(
    state: State<'_, AppState>,
    key_id: String,
    account_id: Option<String>,
) -> Result<SshKeyDescriptor, IpcErrorPayload> {
    let repo = SshKeyRepository::new(&state.storage);
    let mut descriptor = repo
        .get_key(&key_id)
        .map_err(IpcErrorPayload::from)?
        .ok_or_else(|| GitUploadError::Storage(format!("SSH key '{key_id}' not found")))?;

    let acc = account_id.unwrap_or_else(|| "default".to_string());
    let client = state
        .github_client_for_account(&acc)
        .map_err(IpcErrorPayload::from)?;

    client
        .register_ssh_key(&descriptor.title, &descriptor.public_key)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    repo.mark_registered_on_github(&key_id, true)
        .map_err(IpcErrorPayload::from)?;
    descriptor.is_registered_on_github = true;
    Ok(descriptor)
}
