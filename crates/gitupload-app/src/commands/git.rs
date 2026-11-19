//! Git inspection, push, re-push, clone, diff, cancellation, and `.gitignore` commands (`git_*`).

use crate::commands::validate_safe_path;
use crate::events::EventStreamer;
use crate::state::AppState;
use chrono::Utc;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{
    PushHistoryRecord, PushMode, PushPayload, PushResult, RepoInspection,
};
use gitupload_git::coordinator::PushExecutionRequest;
use gitupload_git::{GitCoordinator, GitignoreGenerator, LibGit2Repo, RePushInspector};
use gitupload_storage::{HistoryRepository, SettingsRepository};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Request payload for safe re-pushing an existing `.git` directory.
#[derive(Debug, Deserialize)]
pub struct RePushRequest {
    /// Local repository directory path.
    pub path: String,
    /// Target remote branch (defaults to current branch or `"main"`).
    pub branch: Option<String>,
    /// Commit message when uncommitted changes exist.
    pub commit_msg: Option<String>,
    /// Use `--force-with-lease` instead of strict fast-forward.
    pub force_with_lease: bool,
    /// Account identifier in OS Keyring.
    pub account_id: Option<String>,
}

/// Summary returned when starting a background job.
#[derive(Debug, Serialize)]
pub struct JobStartedPayload {
    /// Unique job identifier.
    pub job_id: String,
}

/// Inspects an existing local directory for `.git`, branch, ahead/behind counts, and dirty status.
#[tauri::command]
pub async fn git_inspect_repo(path: String) -> Result<RepoInspection, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || RePushInspector::inspect(&safe_path))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Computes working tree diff against `HEAD`.
#[tauri::command]
pub async fn git_get_diff(path: String) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    tokio::task::spawn_blocking(move || LibGit2Repo::diff_workdir(&safe_path))
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
        .map_err(IpcErrorPayload::from)
}

/// Executes the full GitUpload push pipeline (creates/reuses GitHub repo, commits via libgit2, pushes safely).
#[tauri::command]
pub async fn git_start_push(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: PushPayload,
    account_id: Option<String>,
) -> Result<PushResult, IpcErrorPayload> {
    let _safe_source = validate_safe_path(&payload.folder_path)?;
    let acc_id = account_id.unwrap_or_else(|| "default".to_string());

    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;

    let token = state
        .get_github_token(&acc_id)
        .map_err(IpcErrorPayload::from)?;
    let gh_client = state
        .github_client_for_account(&acc_id)
        .map_err(IpcErrorPayload::from)?;

    let (repo_name, commit_msg) = GitCoordinator::resolve_push_metadata(&settings, &payload)
        .map_err(IpcErrorPayload::from)?;

    let owner = payload
        .target_org
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
        .or_else(|| {
            if !settings.github_org_name.trim().is_empty() {
                Some(settings.github_org_name.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| settings.github_username.clone());

    if owner.trim().is_empty() {
        return Err(GitUploadError::Config(
            "Vui lòng cài đặt GitHub Username hoặc Organization trong Settings".into(),
        )
        .into());
    }

    let is_org = payload
        .target_org
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty())
        || !settings.github_org_name.trim().is_empty();

    let job_id = Uuid::new_v4().to_string();
    let cancel = CancellationToken::new();
    {
        let mut active = state.active_job_cancel.lock().await;
        if active.is_some() {
            return Err(GitUploadError::SafetyGuard(
                "Một tiến trình push/sync khác đang chạy".into(),
            )
            .into());
        }
        *active = Some(cancel.clone());
    }

    let (log_tx, log_rx) = mpsc::channel::<(String, String)>(256);
    EventStreamer::spawn_log_streamer(app.clone(), job_id.clone(), log_rx);
    EventStreamer::emit_progress(
        &app,
        &job_id,
        "GitHub Repo",
        20,
        "Ensuring remote repository exists...",
    );
    let _ = log_tx
        .send((
            "info".into(),
            format!("Creating or verifying GitHub repository {owner}/{repo_name}..."),
        ))
        .await;

    let created = gh_client
        .create_repository(&owner, &repo_name, is_org, settings.repo_private)
        .await;

    let repo_info = match created {
        Ok(info) => info,
        Err(err) => {
            *state.active_job_cancel.lock().await = None;
            return Err(GitUploadError::from(err).into());
        }
    };

    let remote_url = if repo_info.clone_url.trim().is_empty() {
        format!("https://github.com/{owner}/{repo_name}.git")
    } else {
        repo_info.clone_url.clone()
    };

    let app_for_progress = app.clone();
    let job_id_prog = job_id.clone();
    let log_tx_prog = log_tx.clone();
    let exec_req = PushExecutionRequest {
        payload: payload.clone(),
        settings,
        remote_url: remote_url.clone(),
        repo_name: repo_name.clone(),
        token,
        cancel,
    };

    let push_res = state
        .git
        .execute_push(exec_req, move |pct, msg| {
            EventStreamer::emit_progress(&app_for_progress, &job_id_prog, "Git Sync", pct, &msg);
            let _ = log_tx_prog.try_send(("info".into(), msg));
        })
        .await;

    *state.active_job_cancel.lock().await = None;

    match push_res {
        Ok(res) => {
            let _ = log_tx
                .send((
                    "success".into(),
                    format!("Push succeeded: {}", res.repo_url),
                ))
                .await;
            let history_rec = PushHistoryRecord {
                id: Uuid::new_v4().to_string(),
                account_id: acc_id,
                repo_name: res.repo_name.clone(),
                repo_url: res.repo_url.clone(),
                local_path: payload.folder_path.replace('\\', "/"),
                commit_sha: Some(res.commit_sha.clone()),
                commit_msg,
                files_count: res.files_count,
                status: "SUCCESS".into(),
                error_message: None,
                is_force_with_lease: !payload.safe_sync,
                duration_ms: res.duration_ms,
                created_at: Utc::now(),
            };
            let _ = HistoryRepository::new(&state.storage).save(&history_rec);
            Ok(res)
        }
        Err(err) => {
            let _ = log_tx
                .send(("error".into(), format!("Push failed: {err}")))
                .await;
            Err(err.into())
        }
    }
}

/// Safe Re-Push for an already initialized `.git` repository (`--force-with-lease` optional).
#[tauri::command]
pub async fn git_repush_repo(
    app: AppHandle,
    state: State<'_, AppState>,
    req: RePushRequest,
) -> Result<PushResult, IpcErrorPayload> {
    let safe_path = validate_safe_path(&req.path)?;
    let acc_id = req.account_id.unwrap_or_else(|| "default".to_string());
    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;
    let token = state
        .get_github_token(&acc_id)
        .map_err(IpcErrorPayload::from)?;

    let inspection = RePushInspector::inspect(&safe_path).map_err(IpcErrorPayload::from)?;
    if !inspection.is_git_repo {
        return Err(GitUploadError::Git("Selected folder is not a Git repository".into()).into());
    }

    let branch = req
        .branch
        .or(inspection.current_branch)
        .unwrap_or_else(|| "main".to_string());
    let remote_url = inspection.remote_url.ok_or_else(|| {
        GitUploadError::Git("Repository has no 'origin' remote configured".into())
    })?;
    let repo_name = safe_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string();

    let payload = PushPayload {
        folder_path: safe_path.to_string_lossy().to_string(),
        mode: PushMode::CustomRepository,
        session_val: None,
        ex_val: None,
        mini_project_val: None,
        custom_repo_name: Some(repo_name.clone()),
        use_default_commit: req.commit_msg.is_none(),
        custom_commit_msg: req.commit_msg,
        is_file: false,
        target_org: None,
        safe_sync: !req.force_with_lease,
    };

    let job_id = Uuid::new_v4().to_string();
    let cancel = CancellationToken::new();
    *state.active_job_cancel.lock().await = Some(cancel.clone());

    let app_prog = app.clone();
    let jid = job_id.clone();
    let res = state
        .git
        .execute_push(
            PushExecutionRequest {
                payload,
                settings,
                remote_url,
                repo_name,
                token,
                cancel,
            },
            move |pct, msg| {
                EventStreamer::emit_progress(&app_prog, &jid, &branch, pct, &msg);
            },
        )
        .await;

    *state.active_job_cancel.lock().await = None;
    res.map_err(IpcErrorPayload::from)
}

/// Clones a repository into `dest_path` using in-memory Keyring token callbacks.
#[tauri::command]
pub async fn git_start_clone(
    state: State<'_, AppState>,
    repo_url: String,
    dest_path: String,
    account_id: Option<String>,
) -> Result<(), IpcErrorPayload> {
    let safe_dest = validate_safe_path(&dest_path)?;
    let acc_id = account_id.unwrap_or_else(|| "default".to_string());
    let token = state
        .get_github_token(&acc_id)
        .map_err(IpcErrorPayload::from)?;

    let cancel = CancellationToken::new();
    *state.active_job_cancel.lock().await = Some(cancel.clone());
    let res = state
        .git_engine
        .clone_repo(&repo_url, &safe_dest, token, cancel)
        .await;
    *state.active_job_cancel.lock().await = None;
    res.map_err(IpcErrorPayload::from)
}

/// Cancels the currently active push or clone operation cooperatively.
#[tauri::command]
pub async fn git_cancel_job(state: State<'_, AppState>) -> Result<bool, IpcErrorPayload> {
    let guard = state.active_job_cancel.lock().await;
    if let Some(ref tok) = *guard {
        tok.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Generates a deduplicated `.gitignore` file combining selected language/framework templates.
#[tauri::command]
pub async fn git_generate_gitignore(
    repo_path: String,
    tags: Vec<String>,
) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let tag_refs: Vec<&str> = tags.iter().map(String::as_str).collect();
    let written =
        GitignoreGenerator::write_to_repo(&safe_path, &tag_refs).map_err(IpcErrorPayload::from)?;
    Ok(written.to_string_lossy().to_string())
}
