//! Stale & merged branch topological analysis and protected bulk pruning IPC commands (`git_*_branches`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{PruneBranchesRequest, PruneBranchesResult, StaleBranchInfo};
use gitupload_git::{BranchPruner, StaleBranchAnalyzer};
use gitupload_github::SquashReconciler;
use tauri::State;

/// Inspects local branches against `default_branch` and optionally reconciles GitHub squashed PRs.
#[tauri::command]
pub async fn git_get_stale_branches(
    state: State<'_, AppState>,
    repo_path: String,
    default_branch: Option<String>,
    github_owner: Option<String>,
    github_repo: Option<String>,
) -> Result<Vec<StaleBranchInfo>, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();
    let default_br = default_branch.unwrap_or_else(|| "main".to_string());

    let mut branches = tokio::task::spawn_blocking(move || {
        StaleBranchAnalyzer::analyze_local_branches(&normalized, &default_br)
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    if let (Some(owner), Some(repo)) = (
        github_owner.filter(|s| !s.trim().is_empty()),
        github_repo.filter(|s| !s.trim().is_empty()),
    ) {
        if let Ok(client) = state.github_client_for_account("default") {
            let _ = SquashReconciler::reconcile_squashed_branches(
                &client,
                &owner,
                &repo,
                &mut branches,
            )
            .await;
        }
    }

    Ok(branches)
}

/// Safely prunes selected un-protected branches locally and optionally deletes remote refs.
#[tauri::command]
pub async fn git_prune_branches(
    req: PruneBranchesRequest,
) -> Result<PruneBranchesResult, IpcErrorPayload> {
    let safe_path = validate_safe_path(&req.repo_path)?;
    let safe_req = PruneBranchesRequest {
        repo_path: safe_path.to_string_lossy().to_string(),
        branch_names: req.branch_names,
        delete_remote: req.delete_remote,
        prune_tracking_refs: req.prune_tracking_refs,
    };

    BranchPruner::execute_prune(&safe_req)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}
