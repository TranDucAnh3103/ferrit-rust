//! Existing repository detector and ahead/behind divergence inspector for Re-Push workflows.

use crate::libgit2::LibGit2Repo;
use git2::Repository;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::RepoInspection;
use gitupload_security::sanitizer::sanitize_log_message;
use std::path::Path;

/// Inspects an existing local directory for `.git` state, remote URL, dirty files, and ahead/behind counts.
pub struct RePushInspector;

impl RePushInspector {
    /// Inspects `repo_path` in-process using `libgit2` without modifying the working tree.
    pub fn inspect(repo_path: &Path) -> GitUploadResult<RepoInspection> {
        if !repo_path.join(".git").exists() {
            return Ok(RepoInspection {
                is_git_repo: false,
                current_branch: None,
                remote_url: None,
                ahead: 0,
                behind: 0,
                dirty_files: Vec::new(),
                has_uncommitted_changes: false,
                has_lfs: false,
                has_hooks: false,
            });
        }

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;

        let current_branch = repo.head().ok().and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(String::from)
            } else {
                Some("HEAD".to_string())
            }
        });

        let remote_url = repo
            .find_remote("origin")
            .ok()
            .and_then(|r| r.url().map(sanitize_log_message));

        let (ahead, behind) = compute_ahead_behind(&repo, current_branch.as_deref());
        let dirty_files = LibGit2Repo::status_files(repo_path)?;
        let has_uncommitted_changes = !dirty_files.is_empty();

        let has_lfs = repo_path.join(".gitattributes").is_file()
            && std::fs::read_to_string(repo_path.join(".gitattributes"))
                .map(|c| c.contains("filter=lfs"))
                .unwrap_or(false);

        let has_hooks = repo_path.join(".gitupload").join("hooks.toml").is_file()
            || repo_path
                .join(".git")
                .join("hooks")
                .join("commit-msg")
                .is_file();

        Ok(RepoInspection {
            is_git_repo: true,
            current_branch,
            remote_url,
            ahead,
            behind,
            dirty_files,
            has_uncommitted_changes,
            has_lfs,
            has_hooks,
        })
    }
}

fn compute_ahead_behind(repo: &Repository, branch_name: Option<&str>) -> (usize, usize) {
    let Ok(head) = repo.head() else {
        return (0, 0);
    };
    let Some(local_oid) = head.target() else {
        return (0, 0);
    };

    if let Some(name) = branch_name {
        if let Ok(branch) = repo.find_branch(name, git2::BranchType::Local) {
            if let Ok(upstream) = branch.upstream() {
                if let Some(remote_oid) = upstream.get().target() {
                    return repo
                        .graph_ahead_behind(local_oid, remote_oid)
                        .unwrap_or((0, 0));
                }
            }
        }

        let fallback_ref = format!("refs/remotes/origin/{name}");
        if let Ok(remote_ref) = repo.find_reference(&fallback_ref) {
            if let Some(remote_oid) = remote_ref.target() {
                return repo
                    .graph_ahead_behind(local_oid, remote_oid)
                    .unwrap_or((0, 0));
            }
        }
    }

    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinator::SafePushMode;
    use crate::libgit2::{LibGit2Commit, LibGit2Repo, LibGit2Transport};
    use std::fs;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;

    #[test]
    fn test_repush_inspect() {
        let remote_dir = tempdir().unwrap();
        let local_dir = tempdir().unwrap();
        git2::Repository::init_bare(remote_dir.path()).unwrap();

        let repo = LibGit2Repo::init_or_open(local_dir.path()).unwrap();
        LibGit2Repo::configure_identity(&repo, "Tester", "tester@gitupload.local").unwrap();
        LibGit2Transport::set_clean_remote(&repo, "origin", remote_dir.path().to_str().unwrap())
            .unwrap();
        drop(repo);

        fs::write(local_dir.path().join("README.md"), "base").unwrap();
        LibGit2Commit::stage_and_commit(
            local_dir.path(),
            "Tester",
            "tester@gitupload.local",
            "chore: initial",
        )
        .unwrap();

        let cancel = CancellationToken::new();
        LibGit2Transport::push_branch(
            local_dir.path(),
            "origin",
            "main",
            None,
            SafePushMode::StrictSafe,
            &cancel,
            |_, _| {},
        )
        .unwrap();

        // Fetch to establish remote tracking ref refs/remotes/origin/main
        LibGit2Transport::fetch_remote(local_dir.path(), "origin", None, &cancel).unwrap();

        // Create 2 unpushed local commits
        fs::write(local_dir.path().join("a.txt"), "1").unwrap();
        LibGit2Commit::stage_and_commit(
            local_dir.path(),
            "Tester",
            "tester@gitupload.local",
            "feat: commit 1",
        )
        .unwrap();
        fs::write(local_dir.path().join("b.txt"), "2").unwrap();
        LibGit2Commit::stage_and_commit(
            local_dir.path(),
            "Tester",
            "tester@gitupload.local",
            "feat: commit 2",
        )
        .unwrap();

        let inspection = RePushInspector::inspect(local_dir.path()).unwrap();
        assert!(inspection.is_git_repo);
        assert_eq!(inspection.current_branch.as_deref(), Some("main"));
        assert_eq!(inspection.ahead, 2);
        assert_eq!(inspection.behind, 0);
    }
}
