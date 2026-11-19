#![forbid(unsafe_code)]
//! 1-click automated DevSecOps code patch applicator & changed-file collector.

use git2::Repository;
use gitupload_core::errors::{GitError, GitResult};
use gitupload_core::models::SecurityPatchProposal;
use std::fs;
use std::path::{Component, Path};
use tracing::info;

/// Safely applies AI-proposed security patches inside a repository and gathers changed working files.
pub struct AutomatedPatcher;

impl AutomatedPatcher {
    /// Applies `proposal` onto `relative_file_path` inside `repo_root`, rejecting path traversal.
    pub fn apply_patch(
        repo_root: &Path,
        relative_file_path: &str,
        proposal: &SecurityPatchProposal,
    ) -> GitResult<()> {
        let rel = Path::new(relative_file_path);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
            return Err(GitError::Engine(format!(
                "Refusing unsafe patch path outside repository: '{relative_file_path}'"
            )));
        }

        let full_path = repo_root.join(rel);
        if !full_path.exists() {
            return Err(GitError::Engine(format!(
                "Target file for patch does not exist: {}",
                full_path.display()
            )));
        }

        let content = fs::read_to_string(&full_path)
            .map_err(|e| GitError::Engine(format!("Failed reading file for patching: {e}")))?;

        if !content.contains(&proposal.original_code) {
            return Err(GitError::Engine(
                "Original vulnerable snippet could not be uniquely matched in file. Manual inspection required."
                    .to_string(),
            ));
        }

        let new_content = content.replacen(&proposal.original_code, &proposal.replacement_code, 1);
        fs::write(&full_path, new_content)
            .map_err(|e| GitError::Engine(format!("Failed writing patched file: {e}")))?;

        info!("Applied 1-click security patch to {}", full_path.display());
        Ok(())
    }

    /// Collects modified/untracked UTF-8 file contents in the working tree for pre-push DevSecOps scanning.
    pub fn collect_dirty_files(
        repo_root: &Path,
        max_files: usize,
    ) -> GitResult<Vec<(String, String)>> {
        let repo = Repository::open(repo_root)
            .map_err(|e| GitError::RepoOpen(repo_root.display().to_string(), e.to_string()))?;

        let statuses = repo
            .statuses(None)
            .map_err(|e| GitError::Engine(format!("Failed reading repo statuses: {e}")))?;

        let mut collected = Vec::new();
        for entry in statuses.iter() {
            if collected.len() >= max_files {
                break;
            }
            let st = entry.status();
            if st.is_wt_deleted() || st.is_index_deleted() || st.is_ignored() {
                continue;
            }
            let Some(rel_path) = entry.path() else {
                continue;
            };
            let full_path = repo_root.join(rel_path);
            let Ok(meta) = fs::metadata(&full_path) else {
                continue;
            };
            if !meta.is_file() || meta.len() > 512 * 1024 {
                continue;
            }
            if let Ok(text) = fs::read_to_string(&full_path) {
                collected.push((rel_path.to_string(), text));
            }
        }

        Ok(collected)
    }
}
