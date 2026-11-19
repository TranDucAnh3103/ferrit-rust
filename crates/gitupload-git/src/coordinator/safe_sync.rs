//! Safe Sync push protocol (`--force-with-lease` & collision guard) + Auto-Stash preservation.

use crate::coordinator::repush::RePushInspector;
use crate::libgit2::{LibGit2Repo, LibGit2Transport};
use git2::{Oid, Repository, Signature, StashFlags};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::SecretString;
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// Push safety enforcement mode replacing legacy blind `git push --force`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafePushMode {
    /// Rejects non-fast-forward pushes whenever the remote has unseen commits (`behind > 0`).
    StrictSafe,
    /// Implements `--force-with-lease`: verifies the remote branch OID still matches the local
    /// tracking ref before allowing a history-rewritten push.
    ForceWithLease,
    /// Explicit unsafe override when user confirms overwriting remote history.
    ForceUnsafe,
}

/// RAII helper that stashes uncommitted changes before sync and restores them afterward.
pub struct AutoStashGuard;

impl AutoStashGuard {
    /// Executes `op` while automatically stashing dirty working tree changes if present,
    /// then popping the stash once `op` finishes.
    pub fn with_auto_stash<R, F>(repo_path: &Path, enabled: bool, op: F) -> GitUploadResult<R>
    where
        F: FnOnce() -> GitUploadResult<R>,
    {
        if !enabled {
            return op();
        }

        let dirty = !LibGit2Repo::status_files(repo_path)?.is_empty();
        if !dirty {
            return op();
        }

        let stashed = {
            let mut repo = Repository::open(repo_path)
                .map_err(|e| GitUploadError::Git(format!("Failed opening repo for stash: {e}")))?;
            if repo.head().is_err() {
                false
            } else {
                let sig = repo
                    .signature()
                    .or_else(|_| Signature::now("GitUpload AutoStash", "autostash@gitupload.local"))
                    .map_err(|e| {
                        GitUploadError::Git(format!("Failed creating stash signature: {e}"))
                    })?;
                let res = repo.stash_save(
                    &sig,
                    "gitupload-auto-stash-before-sync",
                    Some(StashFlags::INCLUDE_UNTRACKED),
                );
                res.is_ok()
            }
        };

        let op_result = op();

        if stashed {
            let mut repo = Repository::open(repo_path).map_err(|e| {
                GitUploadError::Git(format!("Failed reopening repo for stash pop: {e}"))
            })?;
            repo.stash_pop(0, None).map_err(|e| {
                GitUploadError::Git(format!(
                    "Auto-stash re-application encountered a conflict: {e}. Your changes remain safely in stash@{{0}}."
                ))
            })?;
        }

        op_result
    }
}

/// Enforces pre-flight remote divergence checks and `--force-with-lease` verification.
pub struct SafeSyncEngine;

impl SafeSyncEngine {
    /// Inspects remote state, optionally auto-stashes dirty edits, verifies divergence / lease,
    /// and executes the `libgit2` push.
    #[allow(clippy::too_many_arguments)]
    pub fn push_with_safety<F>(
        repo_path: &Path,
        remote_name: &str,
        branch: &str,
        token: Option<&SecretString>,
        mode: SafePushMode,
        auto_stash: bool,
        cancel: &CancellationToken,
        on_progress: F,
    ) -> GitUploadResult<()>
    where
        F: FnMut(u32, &str),
    {
        AutoStashGuard::with_auto_stash(repo_path, auto_stash, || {
            Self::verify_remote_safety(repo_path, remote_name, branch, token, mode, cancel)?;
            LibGit2Transport::push_branch(
                repo_path,
                remote_name,
                branch,
                token,
                mode,
                cancel,
                on_progress,
            )
        })
    }

    /// Verifies upstream divergence before pushing.
    pub fn verify_remote_safety(
        repo_path: &Path,
        remote_name: &str,
        branch: &str,
        token: Option<&SecretString>,
        mode: SafePushMode,
        cancel: &CancellationToken,
    ) -> GitUploadResult<()> {
        if mode == SafePushMode::ForceUnsafe {
            return Ok(());
        }

        // Capture expected lease OID from existing tracking ref without mutating it.
        let expected_lease_oid = read_tracking_ref_oid(repo_path, remote_name, branch);
        let live_remote_oid =
            LibGit2Transport::ls_remote_branch_oid(repo_path, remote_name, branch, token, cancel)?;

        match mode {
            SafePushMode::StrictSafe => {
                if let Some(remote_oid) = live_remote_oid {
                    let repo = Repository::open(repo_path).map_err(|e| {
                        GitUploadError::Git(format!("Failed opening repository: {e}"))
                    })?;
                    let local_head_oid = repo.head().ok().and_then(|h| h.target());
                    let is_fast_forward = match local_head_oid {
                        Some(head) => {
                            head == remote_oid
                                || repo.graph_descendant_of(head, remote_oid).unwrap_or(false)
                        }
                        None => false,
                    };
                    if !is_fast_forward {
                        let inspection = RePushInspector::inspect(repo_path)?;
                        return Err(GitUploadError::DivergenceDetected {
                            ahead: inspection.ahead,
                            behind: inspection.behind.max(1),
                            message: format!(
                                "Remote branch '{branch}' has unseen commit(s) ({remote_oid}) not present locally. Pull/merge or resolve conflicts instead of overwriting remote history."
                            ),
                        });
                    }
                }
                let inspection = RePushInspector::inspect(repo_path)?;
                if inspection.behind > 0 {
                    return Err(GitUploadError::DivergenceDetected {
                        ahead: inspection.ahead,
                        behind: inspection.behind,
                        message: format!(
                            "Remote branch '{branch}' has {} unseen commit(s) (divergence detected). Pull/merge or resolve conflicts instead of overwriting remote history.",
                            inspection.behind
                        ),
                    });
                }
            }
            SafePushMode::ForceWithLease => {
                if let (Some(expected), Some(actual)) = (expected_lease_oid, live_remote_oid) {
                    if expected != actual {
                        return Err(GitUploadError::DivergenceDetected {
                            ahead: 0,
                            behind: 1,
                            message: format!(
                                "Safe sync (--force-with-lease) aborted: remote '{branch}' moved from {expected} to {actual} since last fetch."
                            ),
                        });
                    }
                } else if expected_lease_oid.is_none() && live_remote_oid.is_some() {
                    return Err(GitUploadError::DivergenceDetected {
                        ahead: 0,
                        behind: 1,
                        message: format!(
                            "Safe sync (--force-with-lease) aborted: remote branch '{branch}' already exists with unseen commits."
                        ),
                    });
                }
            }
            SafePushMode::ForceUnsafe => {}
        }

        Ok(())
    }
}

fn read_tracking_ref_oid(repo_path: &Path, remote_name: &str, branch: &str) -> Option<Oid> {
    let repo = Repository::open(repo_path).ok()?;
    let ref_name = format!("refs/remotes/{remote_name}/{branch}");
    let target = repo.find_reference(&ref_name).ok().and_then(|r| r.target());
    target
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libgit2::LibGit2Commit;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_safe_sync() {
        let remote_dir = tempdir().unwrap();
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        let bare = Repository::init_bare(remote_dir.path()).unwrap();
        bare.set_head("refs/heads/main").unwrap();
        drop(bare);

        // Client A initializes, commits, pushes
        let repo_a = LibGit2Repo::init_or_open(dir_a.path()).unwrap();
        LibGit2Repo::configure_identity(&repo_a, "User A", "a@gitupload.local").unwrap();
        LibGit2Transport::set_clean_remote(&repo_a, "origin", remote_dir.path().to_str().unwrap())
            .unwrap();
        drop(repo_a);

        fs::write(dir_a.path().join("README.md"), "# V1").unwrap();
        LibGit2Commit::stage_and_commit(dir_a.path(), "User A", "a@gitupload.local", "chore: v1")
            .unwrap();

        let cancel = CancellationToken::new();
        SafeSyncEngine::push_with_safety(
            dir_a.path(),
            "origin",
            "main",
            None,
            SafePushMode::StrictSafe,
            false,
            &cancel,
            |_, _| {},
        )
        .unwrap();

        // Client B clones, commits, pushes V2 ahead of A
        LibGit2Transport::clone_repo(
            remote_dir.path().to_str().unwrap(),
            dir_b.path(),
            None,
            &cancel,
            |_, _| {},
        )
        .unwrap();
        fs::write(dir_b.path().join("README.md"), "# V2 remote").unwrap();
        LibGit2Commit::stage_and_commit(
            dir_b.path(),
            "User B",
            "b@gitupload.local",
            "feat: remote update",
        )
        .unwrap();
        LibGit2Transport::push_branch(
            dir_b.path(),
            "origin",
            "main",
            None,
            SafePushMode::StrictSafe,
            &cancel,
            |_, _| {},
        )
        .unwrap();

        // Client A makes local diverged commit and attempts StrictSafe push -> blocked
        fs::write(dir_a.path().join("local.txt"), "local work").unwrap();
        LibGit2Commit::stage_and_commit(
            dir_a.path(),
            "User A",
            "a@gitupload.local",
            "feat: local work",
        )
        .unwrap();

        let err = SafeSyncEngine::push_with_safety(
            dir_a.path(),
            "origin",
            "main",
            None,
            SafePushMode::StrictSafe,
            false,
            &cancel,
            |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(err, GitUploadError::DivergenceDetected { .. }));

        // ForceWithLease also aborts when remote moved unexpectedly!
        let err_lease = SafeSyncEngine::push_with_safety(
            dir_a.path(),
            "origin",
            "main",
            None,
            SafePushMode::ForceWithLease,
            false,
            &cancel,
            |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(
            err_lease,
            GitUploadError::DivergenceDetected { .. }
        ));
    }

    #[test]
    fn test_autostash() {
        let dir = tempdir().unwrap();
        let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
        LibGit2Repo::configure_identity(&repo, "Tester", "t@gitupload.local").unwrap();
        drop(repo);

        fs::write(dir.path().join("base.txt"), "base").unwrap();
        LibGit2Commit::stage_and_commit(dir.path(), "Tester", "t@gitupload.local", "chore: base")
            .unwrap();

        // Uncommitted dirty edit + untracked file
        fs::write(dir.path().join("base.txt"), "modified dirty work").unwrap();
        fs::write(dir.path().join("untracked.txt"), "draft notes").unwrap();

        AutoStashGuard::with_auto_stash(dir.path(), true, || {
            // Inside sync callback, working tree is clean!
            let dirty_inside = LibGit2Repo::status_files(dir.path())?;
            assert!(dirty_inside.is_empty());
            Ok(())
        })
        .unwrap();

        // After auto-stash guard exits, uncommitted changes are restored seamlessly!
        assert_eq!(
            fs::read_to_string(dir.path().join("base.txt")).unwrap(),
            "modified dirty work"
        );
        assert_eq!(
            fs::read_to_string(dir.path().join("untracked.txt")).unwrap(),
            "draft notes"
        );
    }
}
