#![forbid(unsafe_code)]
//! Auto-updater channel routing and signed payload verification IPC commands (`updater_*`).

use crate::state::AppState;
use crate::updater_channel::{
    fetch_and_verify_manifest, UpdateChannel, UpdateManifestInfo, DEFAULT_UPDATE_SIGNING_PUBKEY_B64,
};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_storage::HooksConfigRepository;
use tauri::State;

const UPDATER_CHANNEL_KEY: &str = "updater_channel";

/// Returns the persisted update release channel (`stable`, `beta`, `nightly`).
#[tauri::command]
pub async fn updater_get_channel(
    state: State<'_, AppState>,
) -> Result<UpdateChannel, IpcErrorPayload> {
    let val = HooksConfigRepository::new(&state.storage)
        .get(UPDATER_CHANNEL_KEY)
        .map_err(IpcErrorPayload::from)?;
    Ok(val
        .map(|s| UpdateChannel::from_str_lossy(&s))
        .unwrap_or_default())
}

/// Persists the selected update release channel (`stable`, `beta`, `nightly`).
#[tauri::command]
pub async fn updater_set_channel(
    state: State<'_, AppState>,
    channel: UpdateChannel,
) -> Result<UpdateChannel, IpcErrorPayload> {
    HooksConfigRepository::new(&state.storage)
        .set(UPDATER_CHANNEL_KEY, channel.as_str())
        .map_err(IpcErrorPayload::from)?;
    Ok(channel)
}

/// Queries the channel manifest endpoint and validates its Ed25519 signature (respecting Air-Gap policy).
#[tauri::command]
pub async fn updater_check_for_updates(
    state: State<'_, AppState>,
    custom_endpoint: Option<String>,
) -> Result<Option<UpdateManifestInfo>, IpcErrorPayload> {
    let channel = updater_get_channel(state).await?;
    let target_os = std::env::consts::OS;
    let current_ver = env!("CARGO_PKG_VERSION");

    let endpoint = custom_endpoint
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| channel.endpoint_url(current_ver, target_os));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| IpcErrorPayload::from(GitUploadError::Internal(e.to_string())))?;

    fetch_and_verify_manifest(&client, &endpoint, DEFAULT_UPDATE_SIGNING_PUBKEY_B64)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::SafetyGuard(e.to_string())))
}
