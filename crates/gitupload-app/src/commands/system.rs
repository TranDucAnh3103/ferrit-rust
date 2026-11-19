//! System utilities: IDE detection/launching, browser links, settings, history, and offline guide (`system_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{AppSettings, PushHistoryRecord};
use gitupload_storage::{HistoryRepository, LegacyImporter, MigrationReport, SettingsRepository};
use gitupload_watcher::RepoWatcher;
use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const EMBEDDED_GUIDE_MD: &str = include_str!("../../resources/GUIDE.md");

/// Detected IDE installation entry.
#[derive(Debug, Clone, Serialize)]
pub struct InstalledIde {
    /// Display name (`VS Code`, `IntelliJ IDEA`, `Cursor`, `Antigravity`).
    pub name: String,
    /// Executable path or CLI name.
    pub path: String,
}

/// Scans host machine for installed IDEs (VS Code, IntelliJ IDEA, Cursor, Antigravity IDE).
#[tauri::command]
pub async fn system_detect_ides() -> Result<Vec<InstalledIde>, IpcErrorPayload> {
    tokio::task::spawn_blocking(detect_ides_sync)
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()).into())
}

/// Launches the specified IDE pointing at `folder_path` with `CREATE_NO_WINDOW`.
#[tauri::command]
pub async fn system_open_in_ide(
    ide_path: String,
    folder_path: String,
) -> Result<(), IpcErrorPayload> {
    let safe_folder = validate_safe_path(&folder_path)?;
    let mut cmd = Command::new(&ide_path);
    cmd.arg(&safe_folder);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.spawn().map_err(GitUploadError::Io)?;
    Ok(())
}

/// Opens a validated `http://` or `https://` URL in the user's default system browser.
#[tauri::command]
pub async fn system_open_browser(url: String) -> Result<(), IpcErrorPayload> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err(GitUploadError::SafetyGuard(
            "Only http:// and https:// URLs can be opened externally".into(),
        )
        .into());
    }
    opener::open(trimmed).map_err(|e| GitUploadError::Internal(e.to_string()).into())
}

/// Opens `folder_path` in the native OS File Explorer.
#[tauri::command]
pub async fn system_open_in_explorer(folder_path: String) -> Result<(), IpcErrorPayload> {
    let safe_folder = validate_safe_path(&folder_path)?;
    opener::open(&safe_folder).map_err(|e| GitUploadError::Internal(e.to_string()).into())
}

/// Loads non-sensitive `AppSettings` from SQLite.
#[tauri::command]
pub async fn system_get_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, IpcErrorPayload> {
    SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)
}

/// Persists non-sensitive `AppSettings` into SQLite.
#[tauri::command]
pub async fn system_save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), IpcErrorPayload> {
    SettingsRepository::new(&state.storage)
        .save_app_settings(&settings)
        .map_err(IpcErrorPayload::from)
}

/// Lists recent push audit history entries from SQLite.
#[tauri::command]
pub async fn system_get_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<PushHistoryRecord>, IpcErrorPayload> {
    HistoryRepository::new(&state.storage)
        .list_recent(limit.unwrap_or(50))
        .map_err(IpcErrorPayload::from)
}

/// Triggers legacy Go/Wails (`config.json`, `history.json`, `.env`) migration if present.
#[tauri::command]
pub async fn system_migrate_legacy(
    state: State<'_, AppState>,
) -> Result<MigrationReport, IpcErrorPayload> {
    LegacyImporter::migrate_if_needed(&state.storage, &state.keyring).map_err(IpcErrorPayload::from)
}

/// Returns embedded offline user documentation rendered as sanitized HTML.
#[tauri::command]
pub async fn system_get_guide() -> Result<String, IpcErrorPayload> {
    let parser = pulldown_cmark::Parser::new(EMBEDDED_GUIDE_MD);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    Ok(html)
}

/// Starts debounced filesystem monitoring on `folder_path`, emitting `watcher-alert` events.
#[tauri::command]
pub async fn system_watch_directory(
    app: AppHandle,
    state: State<'_, AppState>,
    folder_path: String,
) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&folder_path)?;
    {
        let mut guard = state.watcher_cancel.lock().await;
        if let Some(prev) = guard.take() {
            prev.cancel();
        }
        let cancel = CancellationToken::new();
        *guard = Some(cancel.clone());

        let mut watcher = RepoWatcher::new(Duration::from_millis(500));
        watcher.watch_path(&safe_path)?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        watcher.spawn(tx, cancel)?;

        tokio::spawn(async move {
            while let Some(alert) = rx.recv().await {
                let _ = app.emit("watcher-alert", alert);
            }
        });
    }
    Ok(())
}

/// Stops any active debounced filesystem watcher.
#[tauri::command]
pub async fn system_stop_watcher(state: State<'_, AppState>) -> Result<(), IpcErrorPayload> {
    let mut guard = state.watcher_cancel.lock().await;
    if let Some(prev) = guard.take() {
        prev.cancel();
    }
    Ok(())
}

fn detect_ides_sync() -> Vec<InstalledIde> {
    let mut ides = Vec::new();
    let local_app_data = env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let program_files = env::var_os("ProgramFiles").map(PathBuf::from);
    let user_profile = env::var_os("USERPROFILE").map(PathBuf::from);

    // 1. VS Code
    let mut vscode_candidates = Vec::new();
    if let Some(ref la) = local_app_data {
        vscode_candidates.push(
            la.join("Programs")
                .join("Microsoft VS Code")
                .join("Code.exe"),
        );
    }
    if let Some(ref pf) = program_files {
        vscode_candidates.push(pf.join("Microsoft VS Code").join("Code.exe"));
    }
    if let Some(found) = first_existing(&vscode_candidates) {
        ides.push(InstalledIde {
            name: "VS Code".into(),
            path: found.to_string_lossy().to_string(),
        });
    } else if command_succeeds("code", &["--version"]) {
        ides.push(InstalledIde {
            name: "VS Code".into(),
            path: "code".into(),
        });
    }

    // 2. IntelliJ IDEA (Standalone & JetBrains Toolbox)
    let mut idea_found = None;
    if let Some(ref pf) = program_files {
        let jb = pf.join("JetBrains");
        if let Ok(entries) = std::fs::read_dir(jb) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("bin").join("idea64.exe");
                if candidate.is_file() {
                    idea_found = Some(candidate);
                    break;
                }
            }
        }
    }
    if idea_found.is_none() {
        if let Some(ref la) = local_app_data {
            let toolbox = la.join("JetBrains").join("Toolbox").join("apps");
            for edition in ["IDEA-U", "IDEA-C"] {
                let app_dir = toolbox.join(edition);
                if let Ok(channels) = std::fs::read_dir(app_dir) {
                    for ch in channels.flatten() {
                        if let Ok(builds) = std::fs::read_dir(ch.path()) {
                            for b in builds.flatten() {
                                let candidate = b.path().join("bin").join("idea64.exe");
                                if candidate.is_file() {
                                    idea_found = Some(candidate);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(found) = idea_found {
        ides.push(InstalledIde {
            name: "IntelliJ IDEA".into(),
            path: found.to_string_lossy().to_string(),
        });
    }

    // 3. Cursor
    let mut cursor_candidates = Vec::new();
    if let Some(ref la) = local_app_data {
        cursor_candidates.push(la.join("Programs").join("cursor").join("Cursor.exe"));
    }
    if let Some(ref up) = user_profile {
        cursor_candidates.push(
            up.join("AppData")
                .join("Local")
                .join("Programs")
                .join("cursor")
                .join("Cursor.exe"),
        );
    }
    if let Some(found) = first_existing(&cursor_candidates) {
        ides.push(InstalledIde {
            name: "Cursor".into(),
            path: found.to_string_lossy().to_string(),
        });
    } else if command_succeeds("cursor", &["--version"]) {
        ides.push(InstalledIde {
            name: "Cursor".into(),
            path: "cursor".into(),
        });
    }

    // 4. Antigravity IDE
    let mut ag_candidates = Vec::new();
    if let Some(ref up) = user_profile {
        ag_candidates.push(
            up.join(".gemini")
                .join("antigravity-ide")
                .join("Antigravity.exe"),
        );
    }
    if let Some(ref la) = local_app_data {
        ag_candidates.push(
            la.join("Programs")
                .join("antigravity-ide")
                .join("Antigravity.exe"),
        );
    }
    if let Some(found) = first_existing(&ag_candidates) {
        ides.push(InstalledIde {
            name: "Antigravity".into(),
            path: found.to_string_lossy().to_string(),
        });
    }

    ides
}

fn first_existing(paths: &[PathBuf]) -> Option<&Path> {
    paths.iter().find(|p| p.is_file()).map(PathBuf::as_path)
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().is_ok_and(|o| o.status.success())
}
