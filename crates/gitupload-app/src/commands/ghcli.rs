//! GitHub CLI (`gh`) lifecycle detection and sandbox auto-installation commands (`ghcli_*`).

use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::GhCliStatus;
use gitupload_ghcli::GhInstaller;
use tauri::State;

/// Detects `gh.exe` in application sandbox or system `PATH` and checks its semantic version.
#[tauri::command]
pub async fn ghcli_check_status(
    state: State<'_, AppState>,
) -> Result<GhCliStatus, IpcErrorPayload> {
    Ok(state.gh_cli.check_status().await)
}

/// Downloads the official `cli/cli` release archive, verifies SHA256, and extracts `gh.exe`
/// into the zero-admin user sandbox directory (`%APPDATA%/rust-gitupload/bin/`).
#[tauri::command]
pub async fn ghcli_install_silent(
    state: State<'_, AppState>,
) -> Result<GhCliStatus, IpcErrorPayload> {
    let sandbox_dir = state.gh_cli.sandbox_dir().to_path_buf();
    GhInstaller::download_and_extract_sandbox(&sandbox_dir)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;
    Ok(state.gh_cli.check_status().await)
}
