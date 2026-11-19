#![forbid(unsafe_code)]
//! Git workflow coordination: Re-Push inspection, `--force-with-lease` safe sync, auto-stash,
//! and single-file isolation push orchestration.

pub mod repush;
pub mod safe_sync;

pub use repush::RePushInspector;
pub use safe_sync::{AutoStashGuard, SafePushMode, SafeSyncEngine};

use crate::libgit2::{LibGit2Commit, LibGit2Repo, LibGit2Transport};
use chrono::Utc;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{AppSettings, PushPayload, PushResult};
use gitupload_core::naming::{
    resolve_commit_msg_for_file, resolve_repo_name, slugify_repo_name, validate_repository_name,
};
use secrecy::SecretString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tempfile::Builder;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Request parameters for executing a full local-to-remote push job.
pub struct PushExecutionRequest {
    /// Push payload from UI / IPC.
    pub payload: PushPayload,
    /// Active user settings.
    pub settings: AppSettings,
    /// Target remote repository URL (clean, without embedded token).
    pub remote_url: String,
    /// Resolved repository name on GitHub.
    pub repo_name: String,
    /// Optional token wrapped in `SecretString`.
    pub token: Option<SecretString>,
    /// Cooperative cancellation token.
    pub cancel: CancellationToken,
}

/// Coordinates high-level Git operations with mutual exclusion and non-blocking Tokio dispatch.
#[derive(Clone)]
pub struct GitCoordinator {
    push_lock: Arc<Mutex<()>>,
}

impl Default for GitCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl GitCoordinator {
    /// Creates a new `GitCoordinator`.
    pub fn new() -> Self {
        Self {
            push_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Resolves commit message and repository name using legacy-compatible naming & file-parsing rules.
    pub fn resolve_push_metadata(
        settings: &AppSettings,
        payload: &PushPayload,
    ) -> GitUploadResult<(String, String)> {
        let source_path = Path::new(payload.folder_path.trim().trim_matches('"'));
        if !source_path.exists() {
            return Err(GitUploadError::Git(format!(
                "Path does not exist: {}",
                source_path.display()
            )));
        }

        let is_file = payload.is_file || source_path.is_file();
        let commit_msg = if is_file {
            let file_name = source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            let file_stem = source_path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or(file_name);
            resolve_commit_msg_for_file(
                settings,
                payload.use_default_commit,
                payload.custom_commit_msg.as_deref(),
                file_name,
                file_stem,
            )?
        } else if payload.use_default_commit {
            if settings.default_commit_msg.trim().is_empty() {
                "init".to_string()
            } else {
                settings.default_commit_msg.clone()
            }
        } else {
            let custom = payload.custom_commit_msg.as_deref().unwrap_or("").trim();
            if custom.is_empty() {
                return Err(GitUploadError::Config(
                    "Commit message tùy chỉnh không được để trống".into(),
                ));
            }
            custom.to_string()
        };

        let repo_name = if is_file && settings.sync_file_repo_name_with_commit {
            let slug = slugify_repo_name(&commit_msg);
            validate_repository_name(&slug)?;
            slug
        } else {
            resolve_repo_name(settings, payload)?
        };

        Ok((repo_name, commit_msg))
    }

    /// Executes a complete repository push workflow (supporting single-file isolation,
    /// existing `.git` re-push, safe sync `--force-with-lease`, and progress callbacks).
    pub async fn execute_push<F>(
        &self,
        req: PushExecutionRequest,
        mut on_progress: F,
    ) -> GitUploadResult<PushResult>
    where
        F: FnMut(u32, String) + Send + 'static,
    {
        let _guard = self.push_lock.lock().await;
        let started = Instant::now();

        let source_path = PathBuf::from(req.payload.folder_path.trim().trim_matches('"'));
        let (_, commit_msg) = Self::resolve_push_metadata(&req.settings, &req.payload)?;
        let is_file = req.payload.is_file || source_path.is_file();
        let safe_mode = if req.payload.safe_sync {
            SafePushMode::StrictSafe
        } else {
            SafePushMode::ForceWithLease
        };

        let author_name = if req.settings.git_commit_name.trim().is_empty() {
            req.settings.github_username.clone()
        } else {
            req.settings.git_commit_name.clone()
        };
        let author_email = if req.settings.git_commit_email.trim().is_empty() {
            let uname = if author_name.trim().is_empty() {
                "user"
            } else {
                author_name.trim()
            };
            format!("{uname}@users.noreply.github.com")
        } else {
            req.settings.git_commit_email.clone()
        };

        let repo_name = req.repo_name.clone();
        let remote_url = req.remote_url.clone();
        let cancel = req.cancel.clone();
        let token = req.token;
        let auto_stash = req.settings.auto_stash_enabled;

        let result = tokio::task::spawn_blocking(move || {
            if cancel.is_cancelled() {
                return Err(GitUploadError::Cancelled);
            }

            // Hold TempDir guard across blocking block when isolating a single file.
            let _temp_guard;
            let working_dir = if is_file {
                let tmp = Builder::new()
                    .prefix("gitupload-file-")
                    .tempdir()
                    .map_err(GitUploadError::Io)?;
                let file_name = source_path
                    .file_name()
                    .ok_or_else(|| GitUploadError::Git("Invalid file path".into()))?;
                let dest_file = tmp.path().join(file_name);
                fs::copy(&source_path, &dest_file)?;
                let path = tmp.path().to_path_buf();
                _temp_guard = Some(tmp);
                path
            } else {
                _temp_guard = None;
                source_path.clone()
            };

            on_progress(15, "Initializing local Git repository...".into());
            let repo = LibGit2Repo::init_or_open(&working_dir)?;
            LibGit2Repo::configure_identity(&repo, &author_name, &author_email)?;
            LibGit2Repo::ensure_env_ignored(&working_dir)?;
            LibGit2Transport::set_clean_remote(&repo, "origin", &remote_url)?;
            drop(repo);

            on_progress(35, "Staging and committing changes...".into());
            let files_count = LibGit2Repo::status_files(&working_dir)?.len().max(1) as u64;
            let commit_oid = match LibGit2Commit::stage_and_commit_with_options(
                &working_dir,
                &author_name,
                &author_email,
                &commit_msg,
                false,
            ) {
                Ok(oid) => oid,
                Err(_) => {
                    // For existing repos with clean tree (re-push) or empty dirs, use HEAD or empty commit
                    let repo = git2::Repository::open(&working_dir)
                        .map_err(|e| GitUploadError::Git(e.to_string()))?;
                    let head_oid = repo.head().ok().and_then(|head| head.target());
                    drop(repo);
                    if let Some(oid) = head_oid {
                        oid
                    } else {
                        LibGit2Commit::stage_and_commit_with_options(
                            &working_dir,
                            &author_name,
                            &author_email,
                            &commit_msg,
                            true,
                        )?
                    }
                }
            };

            LibGit2Commit::ensure_branch_name(&working_dir, "main")?;

            on_progress(
                55,
                "Checking remote state and synchronizing safely...".into(),
            );
            let sync_res = SafeSyncEngine::push_with_safety(
                &working_dir,
                "origin",
                "main",
                token.as_ref(),
                safe_mode,
                auto_stash,
                &cancel,
                |pct, stage| {
                    let ui_pct = 55 + ((pct * 40) / 100);
                    on_progress(ui_pct, stage.to_string());
                },
            );
            sync_res?;

            on_progress(100, "Push completed cleanly.".into());
            Ok(PushResult {
                repo_name: repo_name.clone(),
                repo_url: remote_url.trim_end_matches(".git").to_string(),
                commit_sha: commit_oid.to_string(),
                branch: "main".into(),
                files_count,
                duration_ms: started.elapsed().as_millis() as u64,
                pushed_at: Utc::now(),
            })
        })
        .await
        .map_err(|e| GitUploadError::Internal(format!("Blocking Git task panicked: {e}")))??;

        Ok(result)
    }

    /// Legacy-compatible helper matching initial skeleton signature.
    pub async fn push(&self, payload: PushPayload) -> GitUploadResult<String> {
        let _guard = self.push_lock.lock().await;
        let path = Path::new(payload.folder_path.trim());
        if !path.exists() {
            return Err(GitUploadError::Git(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }
        Ok("Push validation passed".to_string())
    }
}
