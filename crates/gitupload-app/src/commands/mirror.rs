//! Multi-remote mirroring, endpoint CRUD, health probes, and 60Hz progress IPC commands (`mirror_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_core::errors::IpcErrorPayload;
use gitupload_core::models::{
    MultiRemoteProgressPayload, RemoteEndpoint, RemoteHealthReport, RemotePushResult,
};
use gitupload_git::{MultiRemoteCoordinator, RemoteHealthChecker, SafePushMode};
use gitupload_storage::RemoteRepository;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Lists all saved remote endpoints configured for `repo_path`.
#[tauri::command]
pub async fn mirror_list_remotes(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<Vec<RemoteEndpoint>, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    RemoteRepository::new(&state.storage)
        .list_remotes(&normalized)
        .map_err(IpcErrorPayload::from)
}

/// Inserts or updates a remote endpoint and optionally stores its credential inside OS Credential Manager.
#[tauri::command]
pub async fn mirror_upsert_remote(
    state: State<'_, AppState>,
    repo_path: String,
    remote: RemoteEndpoint,
    token: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();

    if let Some(tok) = token.filter(|t| !t.trim().is_empty()) {
        state
            .remote_vault
            .store_remote_token(&remote.url, &remote.auth_account, tok.trim())
            .map_err(IpcErrorPayload::from)?;
    }

    RemoteRepository::new(&state.storage)
        .upsert_remote(&normalized, &remote)
        .map_err(IpcErrorPayload::from)
}

/// Removes a saved remote endpoint configuration for `repo_path`.
#[tauri::command]
pub async fn mirror_delete_remote(
    state: State<'_, AppState>,
    repo_path: String,
    remote_name: String,
) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    RemoteRepository::new(&state.storage)
        .delete_remote(&normalized, &remote_name)
        .map_err(IpcErrorPayload::from)
}

/// Runs lightweight connectivity handshakes across all configured remotes for `repo_path`.
#[tauri::command]
pub async fn mirror_probe_remotes(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<Vec<RemoteHealthReport>, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    let endpoints = RemoteRepository::new(&state.storage)
        .list_remotes(&normalized)
        .map_err(IpcErrorPayload::from)?;

    let mut reports = Vec::with_capacity(endpoints.len());
    for ep in &endpoints {
        reports.push(RemoteHealthChecker::probe(ep, &state.remote_vault).await);
    }
    Ok(reports)
}

/// Synchronizes `branch` concurrently across all enabled remotes, streaming `multi_remote_progress` events.
#[tauri::command]
pub async fn mirror_push_all(
    app: AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    branch: String,
    force_with_lease: bool,
) -> Result<Vec<RemotePushResult>, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    let endpoints = RemoteRepository::new(&state.storage)
        .list_remotes(&normalized)
        .map_err(IpcErrorPayload::from)?;

    let mode = if force_with_lease {
        SafePushMode::ForceWithLease
    } else {
        SafePushMode::StrictSafe
    };

    let (tx, mut rx) = mpsc::channel::<MultiRemoteProgressPayload>(32);
    let relay_app = app.clone();
    let relay_handle = tokio::spawn(async move {
        let mut last_emit = tokio::time::Instant::now() - Duration::from_millis(50);
        while let Some(payload) = rx.recv().await {
            if payload.overall_percentage >= 100 || last_emit.elapsed() >= Duration::from_millis(30)
            {
                let _ = relay_app.emit("multi_remote_progress", &payload);
                last_emit = tokio::time::Instant::now();
            }
        }
    });

    let cancel = CancellationToken::new();
    let results = MultiRemoteCoordinator::push_all(
        &safe_path,
        branch.trim(),
        &endpoints,
        mode,
        &state.remote_vault,
        Some(tx),
        cancel,
    )
    .await;

    let _ = relay_handle.await;
    Ok(results)
}
