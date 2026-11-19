//! GitHub API commands for Orgs, Repositories, Pull Requests, Actions, Issues, and Releases.

use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{
    BranchProtectionRule, ContentInfo, FileContentInfo, GitHubOrg, GitHubProfile, TokenReport,
};
use gitupload_github::issues_releases::{CreateIssueRequest, CreateReleaseRequest, IssueSummary};
use gitupload_github::pr::{CreatePullRequest, MergePullRequest, PullRequestInfo};
use gitupload_github::repos::SearchRepositoriesResult;
use gitupload_github::WorkflowRun;
use tauri::State;

/// Lists all GitHub Organizations affiliated with the active account.
#[tauri::command]
pub async fn org_list(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<Vec<GitHubOrg>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .list_user_orgs()
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Searches repositories with pagination and sorting.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn repo_search(
    state: State<'_, AppState>,
    owner: String,
    query: String,
    is_org: bool,
    page: usize,
    page_size: usize,
    sort_by: String,
    account_id: Option<String>,
) -> Result<SearchRepositoriesResult, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .search_repositories(
            &owner,
            &query,
            is_org,
            page.max(1),
            page_size.clamp(1, 100),
            &sort_by,
        )
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Deletes a remote repository on GitHub.
#[tauri::command]
pub async fn repo_delete(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    account_id: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .delete_repository(&owner, &repo)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Lists remote repository files/directories at `path`.
#[tauri::command]
pub async fn repo_contents(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    path: String,
    account_id: Option<String>,
) -> Result<Vec<ContentInfo>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_repo_contents(&owner, &repo, &path)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Fetches sanitized HTML README for `{owner}/{repo}`.
#[tauri::command]
pub async fn repo_readme(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    account_id: Option<String>,
) -> Result<Option<String>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_repo_readme(&owner, &repo)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Reads Base64 file contents and SHA for in-app remote editing.
#[tauri::command]
pub async fn repo_file_get(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    path: String,
    account_id: Option<String>,
) -> Result<FileContentInfo, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_repo_file(&owner, &repo, &path)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Commits remote file edits via GitHub Contents API.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn repo_file_update(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    path: String,
    content_base64: String,
    sha: String,
    commit_msg: String,
    account_id: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .update_repo_file(&owner, &repo, &path, &content_base64, &sha, &commit_msg)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Fetches authenticated GitHub profile info.
#[tauri::command]
pub async fn repo_get_profile(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<GitHubProfile, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_profile()
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Fetches token scopes, rate-limits, and account statistics.
#[tauri::command]
pub async fn repo_token_report(
    state: State<'_, AppState>,
    account_id: Option<String>,
) -> Result<TokenReport, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_token_report()
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Inspects GitHub branch protection rules for a branch.
#[tauri::command]
pub async fn repo_branch_protection(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    branch: String,
    account_id: Option<String>,
) -> Result<BranchProtectionRule, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .get_branch_protection(&owner, &repo, &branch)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Lists Pull Requests for `{owner}/{repo}`.
#[tauri::command]
pub async fn pr_list(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    pr_state: Option<String>,
    account_id: Option<String>,
) -> Result<Vec<PullRequestInfo>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .list_pull_requests(&owner, &repo, pr_state.as_deref().unwrap_or("open"))
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Creates a standard or Draft Pull Request.
#[tauri::command]
pub async fn pr_create(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    req: CreatePullRequest,
    account_id: Option<String>,
) -> Result<PullRequestInfo, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .create_pull_request(&owner, &repo, &req)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Merges a Pull Request using `"merge"`, `"squash"`, or `"rebase"`.
#[tauri::command]
pub async fn pr_merge(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    pr_number: u64,
    req: MergePullRequest,
    account_id: Option<String>,
) -> Result<bool, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .merge_pull_request(&owner, &repo, pr_number, &req)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Lists recent GitHub Actions workflow runs.
#[tauri::command]
pub async fn actions_list_runs(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    account_id: Option<String>,
) -> Result<Vec<WorkflowRun>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .list_workflow_runs(&owner, &repo)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Triggers rerun of failed jobs for a workflow run.
#[tauri::command]
pub async fn actions_rerun_failed(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    run_id: u64,
    account_id: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .rerun_failed_jobs(&owner, &repo, run_id)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Cancels an active workflow run.
#[tauri::command]
pub async fn actions_cancel_run(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    run_id: u64,
    account_id: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .cancel_workflow_run(&owner, &repo, run_id)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Lists repository issues.
#[tauri::command]
pub async fn issues_list(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    account_id: Option<String>,
) -> Result<Vec<IssueSummary>, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .list_issues(&owner, &repo)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Creates a GitHub Issue.
#[tauri::command]
pub async fn issues_create(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    req: CreateIssueRequest,
    account_id: Option<String>,
) -> Result<IssueSummary, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .create_issue(&owner, &repo, &req)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Publishes a GitHub Release.
#[tauri::command]
pub async fn releases_create(
    state: State<'_, AppState>,
    owner: String,
    repo: String,
    req: CreateReleaseRequest,
    account_id: Option<String>,
) -> Result<u64, IpcErrorPayload> {
    let client = state.github_client_for_account(account_id.as_deref().unwrap_or("default"))?;
    client
        .create_release(&owner, &repo, &req)
        .await
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}
