//! Autonomous background sync schedule, Windows power awareness, and `.bundle` backup commands (`sync_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{AutoSyncOutcome, PowerState, SyncScheduleConfig};
use gitupload_git::{AutoSyncEngine, GitBackupEngine};
use gitupload_storage::HooksConfigRepository;
use gitupload_watcher::{PowerMonitor, ScheduledSyncDaemon};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

const SYNC_CONFIG_KEY: &str = "sync_schedule_config";

/// Returns the live operating system battery / AC power state.
#[tauri::command]
pub async fn sync_get_power_state() -> Result<PowerState, IpcErrorPayload> {
    Ok(PowerMonitor::get_current_power_state())
}

/// Loads persisted autonomous background sync configuration.
#[tauri::command]
pub async fn sync_get_config(
    state: State<'_, AppState>,
) -> Result<SyncScheduleConfig, IpcErrorPayload> {
    let repo = HooksConfigRepository::new(&state.storage);
    match repo.get(SYNC_CONFIG_KEY).map_err(IpcErrorPayload::from)? {
        Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
        None => Ok(SyncScheduleConfig::default()),
    }
}

/// Saves background sync policy and restarts the energy-aware Tokio cron daemon.
#[tauri::command]
pub async fn sync_save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: SyncScheduleConfig,
) -> Result<(), IpcErrorPayload> {
    let serialized = serde_json::to_string(&config)
        .map_err(|e| IpcErrorPayload::from(GitUploadError::Serialization(e)))?;
    HooksConfigRepository::new(&state.storage)
        .set(SYNC_CONFIG_KEY, &serialized)
        .map_err(IpcErrorPayload::from)?;

    let mut guard = state.sync_daemon_cancel.lock().await;
    if let Some(prev) = guard.take() {
        prev.cancel();
    }

    if config.enabled {
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());
        drop(guard);

        let app_clone = app.clone();
        ScheduledSyncDaemon::spawn_daemon(config, cancel, move || {
            let emitter = app_clone.clone();
            async move {
                let _ = emitter.emit(
                    "background_sync_tick",
                    serde_json::json!({
                        "power_state": PowerMonitor::get_current_power_state(),
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    }),
                );
                Ok(())
            }
        });
    }

    Ok(())
}

/// Triggers an immediate autonomous fetch + safe fast-forward sync check on a local repository.
#[tauri::command]
pub async fn sync_run_now(
    state: State<'_, AppState>,
    repo_path: String,
    remote_name: Option<String>,
    branch_name: Option<String>,
) -> Result<AutoSyncOutcome, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    let remote = remote_name.unwrap_or_else(|| "origin".to_string());
    let branch = branch_name.unwrap_or_else(|| "main".to_string());
    let token = state.get_github_token("default").ok().flatten();

    tokio::task::spawn_blocking(move || {
        AutoSyncEngine::sync_repository(&normalized, &remote, &branch, token.as_ref())
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Generates a standalone cold-storage `.bundle` archive of `repo_path` in the app backups folder.
#[tauri::command]
pub async fn sync_create_bundle(
    app: AppHandle,
    repo_path: String,
    repo_name: Option<String>,
) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let derived_name = repo_name
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            safe_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "repository".to_string());

    let backup_root = app
        .path()
        .app_local_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("GitUpload")
        .join("backups");

    let created = GitBackupEngine::create_repo_bundle(&safe_path, &backup_root, &derived_name)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    Ok(created.to_string_lossy().to_string())
}
