#![forbid(unsafe_code)]
//! Bulk branch pruning engine enforcing mandatory branch protection guardrails.

use crate::stale_analyzer::StaleBranchAnalyzer;
use git2::{BranchType, Repository};
use gitupload_core::errors::{GitError, GitResult};
use gitupload_core::models::{PruneBranchesRequest, PruneBranchesResult};
use tracing::info;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Deletes selected merged/stale branches locally and optionally on `origin` while blocking protected refs.
pub struct BranchPruner;

impl BranchPruner {
    /// Executes bulk branch pruning per `req` while strictly blocking protected branches.
    pub async fn execute_prune(req: &PruneBranchesRequest) -> GitResult<PruneBranchesResult> {
        let repo = Repository::open(&req.repo_path)
            .map_err(|e| GitError::RepoOpen(req.repo_path.clone(), e.to_string()))?;

        let mut deleted_local = 0usize;
        let mut deleted_remote = 0usize;
        let mut failed: Vec<(String, String)> = Vec::new();

        for name in &req.branch_names {
            if StaleBranchAnalyzer::is_protected_branch(name) {
                failed.push((
                    name.clone(),
                    "Branch is protected and cannot be deleted".to_string(),
                ));
                continue;
            }

            if let Ok(mut branch) = repo.find_branch(name, BranchType::Local) {
                if let Err(e) = branch.delete() {
                    failed.push((name.clone(), format!("Local deletion failed: {e}")));
                    continue;
                }
                deleted_local += 1;
            }

            if req.delete_remote {
                let mut cmd = tokio::process::Command::new("git");
                cmd.current_dir(&req.repo_path)
                    .args(["push", "origin", "--delete", name]);
                #[cfg(windows)]
                cmd.creation_flags(CREATE_NO_WINDOW);

                match cmd.output().await {
                    Ok(out) if out.status.success() => deleted_remote += 1,
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr);
                        failed.push((
                            name.clone(),
                            format!("Remote deletion failed: {}", err.trim()),
                        ));
                    }
                    Err(e) => failed.push((name.clone(), format!("Process spawn failed: {e}"))),
                }
            }
        }

        if req.prune_tracking_refs {
            let mut cmd = tokio::process::Command::new("git");
            cmd.current_dir(&req.repo_path)
                .args(["remote", "prune", "origin"]);
            #[cfg(windows)]
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.output().await;
        }

        info!(
            "Pruned {} local and {} remote branches in '{}'",
            deleted_local, deleted_remote, req.repo_path
        );
        Ok(PruneBranchesResult {
            deleted_local_count: deleted_local,
            deleted_remote_count: deleted_remote,
            failed_deletions: failed,
        })
    }
}
