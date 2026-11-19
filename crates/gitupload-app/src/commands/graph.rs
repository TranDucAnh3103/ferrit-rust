//! Interactive Git DAG Graph and contextual commit mutation commands (`graph_*`).

use crate::commands::validate_safe_path;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{GitDagResponse, ResetMode};
use gitupload_git::{GitGraphActions, GitGraphEngine};

/// Computes topological DAG commit nodes, swimlane coordinates, edges, and ref badges.
#[tauri::command]
pub async fn graph_get_commits(
    path: String,
    limit: Option<usize>,
) -> Result<GitDagResponse, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    let max_commits = limit.unwrap_or(500).clamp(1, 50_000);
    tokio::task::spawn_blocking(move || GitGraphEngine::build_dag_full(&safe_path, max_commits))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Cherry-picks `oid` onto current `HEAD`.
#[tauri::command]
pub async fn graph_cherry_pick(path: String, oid: String) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || GitGraphActions::cherry_pick(&safe_path, &oid))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Creates an inverse revert commit for `oid`.
#[tauri::command]
pub async fn graph_revert(path: String, oid: String) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || GitGraphActions::revert_commit(&safe_path, &oid))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Resets active `HEAD` to `oid` with mode `"Soft"`, `"Mixed"`, or `"Hard"`.
#[tauri::command]
pub async fn graph_reset(path: String, oid: String, mode: String) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    let parsed_mode = match mode.trim().to_ascii_lowercase().as_str() {
        "soft" => ResetMode::Soft,
        "mixed" => ResetMode::Mixed,
        "hard" => ResetMode::Hard,
        other => {
            return Err(GitUploadError::Config(format!("Invalid reset mode '{other}'")).into())
        }
    };

    tokio::task::spawn_blocking(move || {
        GitGraphActions::reset_to_commit(&safe_path, &oid, parsed_mode)
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(IpcErrorPayload::from)
}

/// Checks out a branch name or commit OID.
#[tauri::command]
pub async fn graph_checkout(path: String, target_ref: String) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || GitGraphActions::checkout_ref(&safe_path, &target_ref))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Creates a new branch at `commit_oid` (or `HEAD`).
#[tauri::command]
pub async fn graph_create_branch(
    path: String,
    branch_name: String,
    commit_oid: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || {
        GitGraphActions::create_branch(&safe_path, &branch_name, commit_oid.as_deref())
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(IpcErrorPayload::from)
}
