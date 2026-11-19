//! Sub-3ms in-process staging and commit creation via `libgit2`.

use git2::{IndexAddOption, Oid, Repository, Signature};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use std::path::Path;

/// High-speed in-process staging and commit operations.
pub struct LibGit2Commit;

impl LibGit2Commit {
    /// Stages all non-ignored files in the working tree and creates a commit on `HEAD`.
    pub fn stage_and_commit(
        repo_path: &Path,
        author_name: &str,
        author_email: &str,
        message: &str,
    ) -> GitUploadResult<Oid> {
        Self::stage_and_commit_with_options(repo_path, author_name, author_email, message, false)
    }

    /// Stages all changes and creates a commit, optionally permitting empty commits.
    pub fn stage_and_commit_with_options(
        repo_path: &Path,
        author_name: &str,
        author_email: &str,
        message: &str,
        allow_empty: bool,
    ) -> GitUploadResult<Oid> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?;

        let mut index = repo
            .index()
            .map_err(|e| GitUploadError::Git(format!("Failed to open index: {e}")))?;
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(|e| GitUploadError::Git(format!("Failed to stage files: {e}")))?;
        index
            .write()
            .map_err(|e| GitUploadError::Git(format!("Failed to write index: {e}")))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| GitUploadError::Git(format!("Failed to write tree: {e}")))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(format!("Failed to find tree: {e}")))?;

        let safe_name = if author_name.trim().is_empty() {
            "GitUpload User"
        } else {
            author_name.trim()
        };
        let safe_email = if author_email.trim().is_empty() {
            "user@users.noreply.github.com"
        } else {
            author_email.trim()
        };

        let sig = Signature::now(safe_name, safe_email)
            .map_err(|e| GitUploadError::Git(format!("Invalid signature: {e}")))?;

        let parent_commit = match repo.head() {
            Ok(head_ref) => head_ref.peel_to_commit().ok(),
            Err(_) => None,
        };

        if !allow_empty {
            if let Some(ref parent) = parent_commit {
                if parent.tree_id() == tree_id {
                    return Err(GitUploadError::Git(
                        "Nothing to commit: working tree matches HEAD".into(),
                    ));
                }
            }
        }

        let parents: Vec<&git2::Commit<'_>> = match &parent_commit {
            Some(c) => vec![c],
            None => vec![],
        };

        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(|e| GitUploadError::Git(format!("Failed to commit: {e}")))?;

        Ok(commit_oid)
    }

    /// Renames or sets the active local branch to `target_branch` (e.g. `"main"`).
    pub fn ensure_branch_name(repo_path: &Path, target_branch: &str) -> GitUploadResult<()> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?;
        if let Ok(head) = repo.head() {
            if head.is_branch() {
                if let Some(current_name) = head.shorthand() {
                    if current_name != target_branch {
                        let mut branch = repo
                            .find_branch(current_name, git2::BranchType::Local)
                            .map_err(|e| GitUploadError::Git(e.to_string()))?;
                        branch.rename(target_branch, true).map_err(|e| {
                            GitUploadError::Git(format!("Failed renaming branch: {e}"))
                        })?;
                    }
                }
            }
        }
        Ok(())
    }
}
