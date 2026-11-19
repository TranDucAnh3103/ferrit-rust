//! Contextual Git graph actions: cherry-pick, revert, reset (Soft/Mixed/Hard), checkout, branch, and tag.

use git2::{build::CheckoutBuilder, Oid, Repository, ResetType, Signature};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::ResetMode;
use std::path::Path;

/// Direct Git mutations invocable from commit nodes on the interactive DAG graph.
pub struct GitGraphActions;

impl GitGraphActions {
    /// Applies the changes introduced by `commit_oid` onto `HEAD` and creates a new commit.
    pub fn cherry_pick(repo_path: &Path, commit_oid: &str) -> GitUploadResult<String> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let oid = Oid::from_str(commit_oid)
            .map_err(|e| GitUploadError::Git(format!("Invalid OID '{commit_oid}': {e}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| GitUploadError::Git(format!("Commit '{commit_oid}' not found: {e}")))?;

        repo.cherrypick(&commit, None)
            .map_err(|e| GitUploadError::Git(format!("Cherry-pick failed: {e}")))?;

        let mut index = repo
            .index()
            .map_err(|e| GitUploadError::Git(format!("Failed reading index: {e}")))?;
        if index.has_conflicts() {
            return Err(GitUploadError::Git(
                "Cherry-pick resulted in merge conflicts that require manual resolution".into(),
            ));
        }

        let tree_id = index
            .write_tree()
            .map_err(|e| GitUploadError::Git(format!("Failed writing tree: {e}")))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(e.to_string()))?;
        let head_commit = repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .map_err(|e| GitUploadError::Git(format!("HEAD has no commit: {e}")))?;

        let sig = resolve_signature(&repo)?;
        let msg = commit.message().unwrap_or("cherry-pick commit");
        let new_oid = repo
            .commit(Some("HEAD"), &sig, &sig, msg, &tree, &[&head_commit])
            .map_err(|e| GitUploadError::Git(format!("Failed committing cherry-pick: {e}")))?;

        let _ = repo.cleanup_state();
        Ok(new_oid.to_string())
    }

    /// Creates a new commit inverting the changes introduced by `commit_oid`.
    pub fn revert_commit(repo_path: &Path, commit_oid: &str) -> GitUploadResult<String> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let oid = Oid::from_str(commit_oid)
            .map_err(|e| GitUploadError::Git(format!("Invalid OID '{commit_oid}': {e}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| GitUploadError::Git(format!("Commit '{commit_oid}' not found: {e}")))?;

        repo.revert(&commit, None)
            .map_err(|e| GitUploadError::Git(format!("Revert failed: {e}")))?;

        let mut index = repo
            .index()
            .map_err(|e| GitUploadError::Git(format!("Failed reading index: {e}")))?;
        if index.has_conflicts() {
            return Err(GitUploadError::Git(
                "Revert resulted in conflicts that require resolution".into(),
            ));
        }

        let tree_id = index
            .write_tree()
            .map_err(|e| GitUploadError::Git(e.to_string()))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(e.to_string()))?;
        let head_commit = repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .map_err(|e| GitUploadError::Git(format!("HEAD has no commit: {e}")))?;

        let sig = resolve_signature(&repo)?;
        let summary = commit.summary().unwrap_or(commit_oid);
        let msg = format!("Revert \"{summary}\"\n\nThis reverts commit {oid}.");
        let new_oid = repo
            .commit(Some("HEAD"), &sig, &sig, &msg, &tree, &[&head_commit])
            .map_err(|e| GitUploadError::Git(format!("Failed committing revert: {e}")))?;

        let _ = repo.cleanup_state();
        Ok(new_oid.to_string())
    }

    /// Resets active `HEAD` to `commit_oid` using Soft, Mixed, or Hard mode.
    pub fn reset_to_commit(
        repo_path: &Path,
        commit_oid: &str,
        mode: ResetMode,
    ) -> GitUploadResult<()> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let oid = Oid::from_str(commit_oid)
            .map_err(|e| GitUploadError::Git(format!("Invalid OID '{commit_oid}': {e}")))?;
        let obj = repo
            .find_object(oid, None)
            .map_err(|e| GitUploadError::Git(format!("Object '{commit_oid}' not found: {e}")))?;

        let reset_type = match mode {
            ResetMode::Soft => ResetType::Soft,
            ResetMode::Mixed => ResetType::Mixed,
            ResetMode::Hard => ResetType::Hard,
        };

        repo.reset(&obj, reset_type, None)
            .map_err(|e| GitUploadError::Git(format!("Reset failed: {e}")))?;
        Ok(())
    }

    /// Checks out a local branch or raw commit OID (detached HEAD).
    pub fn checkout_ref(repo_path: &Path, target_ref: &str) -> GitUploadResult<()> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts.safe();

        if let Ok((object, reference)) = repo.revparse_ext(target_ref) {
            repo.checkout_tree(&object, Some(&mut checkout_opts))
                .map_err(|e| GitUploadError::Git(format!("Checkout tree failed: {e}")))?;
            if let Some(ref_obj) = reference {
                if let Some(ref_name) = ref_obj.name() {
                    repo.set_head(ref_name)
                        .map_err(|e| GitUploadError::Git(format!("Failed setting HEAD: {e}")))?;
                    return Ok(());
                }
            }
            repo.set_head_detached(object.id())
                .map_err(|e| GitUploadError::Git(format!("Failed detaching HEAD: {e}")))?;
            return Ok(());
        }

        Err(GitUploadError::Git(format!(
            "Target reference '{target_ref}' not found"
        )))
    }

    /// Creates a new local branch pointing to `commit_oid` (or `HEAD` when omitted).
    pub fn create_branch(
        repo_path: &Path,
        branch_name: &str,
        commit_oid: Option<&str>,
    ) -> GitUploadResult<()> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let target_commit = match commit_oid {
            Some(oid_str) => {
                let oid = Oid::from_str(oid_str)
                    .map_err(|e| GitUploadError::Git(format!("Invalid OID: {e}")))?;
                repo.find_commit(oid)
                    .map_err(|e| GitUploadError::Git(e.to_string()))?
            }
            None => repo
                .head()
                .and_then(|h| h.peel_to_commit())
                .map_err(|e| GitUploadError::Git(e.to_string()))?,
        };
        repo.branch(branch_name, &target_commit, false)
            .map_err(|e| GitUploadError::Git(format!("Failed creating branch: {e}")))?;
        Ok(())
    }

    /// Creates an annotated or lightweight tag at `commit_oid` (or `HEAD`).
    pub fn create_tag(
        repo_path: &Path,
        tag_name: &str,
        commit_oid: Option<&str>,
        message: Option<&str>,
    ) -> GitUploadResult<()> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let target_obj = match commit_oid {
            Some(oid_str) => {
                let oid = Oid::from_str(oid_str)
                    .map_err(|e| GitUploadError::Git(format!("Invalid OID: {e}")))?;
                repo.find_object(oid, None)
                    .map_err(|e| GitUploadError::Git(e.to_string()))?
            }
            None => repo
                .head()
                .and_then(|h| h.peel(git2::ObjectType::Commit))
                .map_err(|e| GitUploadError::Git(e.to_string()))?,
        };

        if let Some(msg) = message {
            let sig = resolve_signature(&repo)?;
            repo.tag(tag_name, &target_obj, &sig, msg, false)
                .map_err(|e| GitUploadError::Git(format!("Failed creating annotated tag: {e}")))?;
        } else {
            repo.tag_lightweight(tag_name, &target_obj, false)
                .map_err(|e| {
                    GitUploadError::Git(format!("Failed creating lightweight tag: {e}"))
                })?;
        }

        Ok(())
    }
}

fn resolve_signature(repo: &Repository) -> GitUploadResult<Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        let name = sig.name().unwrap_or("GitUpload User").to_string();
        let email = sig
            .email()
            .unwrap_or("user@users.noreply.github.com")
            .to_string();
        return Signature::now(&name, &email)
            .map_err(|e| GitUploadError::Git(format!("Invalid signature: {e}")));
    }
    Signature::now("GitUpload User", "user@users.noreply.github.com")
        .map_err(|e| GitUploadError::Git(format!("Invalid signature: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libgit2::{LibGit2Commit, LibGit2Repo};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_cherry_pick() {
        let dir = tempdir().unwrap();
        let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
        LibGit2Repo::configure_identity(&repo, "Tester", "tester@gitupload.local").unwrap();

        fs::write(dir.path().join("base.txt"), "base").unwrap();
        let base_oid = LibGit2Commit::stage_and_commit(
            dir.path(),
            "Tester",
            "tester@gitupload.local",
            "chore: base",
        )
        .unwrap();

        GitGraphActions::create_branch(dir.path(), "feature", Some(&base_oid.to_string())).unwrap();
        GitGraphActions::checkout_ref(dir.path(), "feature").unwrap();

        fs::write(dir.path().join("feat.txt"), "feature content").unwrap();
        let feat_oid = LibGit2Commit::stage_and_commit(
            dir.path(),
            "Tester",
            "tester@gitupload.local",
            "feat: add feat.txt",
        )
        .unwrap();

        GitGraphActions::checkout_ref(dir.path(), "main").unwrap();
        assert!(!dir.path().join("feat.txt").exists());

        let picked_oid = GitGraphActions::cherry_pick(dir.path(), &feat_oid.to_string()).unwrap();
        assert!(!picked_oid.is_empty());
        assert!(dir.path().join("feat.txt").exists());
    }
}
