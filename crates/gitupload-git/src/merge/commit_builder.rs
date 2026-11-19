#![forbid(unsafe_code)]
//! Atomic 2-parent merge commit builder writing resolved conflict files cleanly through `git2::Index`.

use crate::merge::calculator::MergeCalculator;
use crate::merge::resolver::ConflictResolver;
use git2::Repository;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::FileResolutionRequest;
use std::collections::HashMap;
use std::path::Path;

/// Writes user-resolved conflict buffers into the working tree & index and records a 2-parent merge commit.
pub struct CommitBuilder;

impl CommitBuilder {
    /// Applies every [`FileResolutionRequest`], validates zero residual conflicts, stages resolved files,
    /// and creates an atomic 2-parent merge commit advancing `HEAD`. Returns the new commit SHA hex.
    pub fn finalize_merge(
        repo_path: &Path,
        incoming_ref: &str,
        resolutions: &[FileResolutionRequest],
        commit_message: Option<&str>,
    ) -> GitUploadResult<String> {
        let session = MergeCalculator::detect_conflicts(repo_path, incoming_ref)?;
        let req_map: HashMap<&str, &FileResolutionRequest> = resolutions
            .iter()
            .map(|r| (r.file_path.as_str(), r))
            .collect();

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let workdir = repo.workdir().ok_or_else(|| {
            GitUploadError::Git("Cannot finalize merge inside a bare repository".to_string())
        })?;

        let ours_commit = repo
            .head()
            .map_err(|e| GitUploadError::Git(format!("HEAD lookup failed: {e}")))?
            .peel_to_commit()
            .map_err(|e| GitUploadError::Git(format!("HEAD commit error: {e}")))?;
        let theirs_commit = repo
            .revparse_single(incoming_ref.trim())
            .map_err(|e| GitUploadError::Git(format!("Incoming ref error: {e}")))?
            .peel_to_commit()
            .map_err(|e| GitUploadError::Git(format!("Incoming commit error: {e}")))?;

        let base_tree = match repo
            .merge_base(ours_commit.id(), theirs_commit.id())
            .ok()
            .and_then(|id| repo.find_commit(id).ok())
            .and_then(|c| c.tree().ok())
        {
            Some(tree) => tree,
            None => {
                let empty_id = repo
                    .treebuilder(None)
                    .map_err(|e| GitUploadError::Git(format!("Empty treebuilder failed: {e}")))?
                    .write()
                    .map_err(|e| GitUploadError::Git(format!("Empty tree write failed: {e}")))?;
                repo.find_tree(empty_id)
                    .map_err(|e| GitUploadError::Git(format!("Empty tree lookup failed: {e}")))?
            }
        };

        let ours_tree = ours_commit
            .tree()
            .map_err(|e| GitUploadError::Git(format!("Ours tree error: {e}")))?;
        let theirs_tree = theirs_commit
            .tree()
            .map_err(|e| GitUploadError::Git(format!("Theirs tree error: {e}")))?;

        let merged_index = repo
            .merge_trees(&base_tree, &ours_tree, &theirs_tree, None)
            .map_err(|e| GitUploadError::Git(format!("3-way merge_trees failed: {e}")))?;

        for file_desc in &session.conflicting_files {
            let req = req_map.get(file_desc.file_path.as_str()).ok_or_else(|| {
                GitUploadError::Git(format!(
                    "Missing resolution plan for conflicted file '{}'",
                    file_desc.file_path
                ))
            })?;

            let resolved_text = if file_desc.is_binary {
                req.custom_content.clone().ok_or_else(|| {
                    GitUploadError::Git(format!(
                        "Binary file '{}' requires explicit custom resolution",
                        file_desc.file_path
                    ))
                })?
            } else {
                ConflictResolver::apply_resolutions(
                    &file_desc.raw_conflicted_text,
                    &file_desc.hunks,
                    req,
                )?
            };

            let target_file = workdir.join(&file_desc.file_path);
            if let Some(parent) = target_file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target_file, resolved_text.as_bytes())?;
        }

        // Write cleanly via repository index seeded from merged_index resolved entries
        let mut index = repo
            .index()
            .map_err(|e| GitUploadError::Git(format!("Repository index error: {e}")))?;
        index
            .clear()
            .map_err(|e| GitUploadError::Git(e.to_string()))?;

        // Copy all non-conflicting stage-0 entries from merged_index into index
        for entry in merged_index.iter() {
            let stage = (entry.flags & 0x3000) >> 12;
            if stage == 0 {
                index
                    .add(&entry)
                    .map_err(|e| GitUploadError::Git(e.to_string()))?;
                // Also materialize non-conflicting merged files to workdir
                if let Ok(blob) = repo.find_blob(entry.id) {
                    let rel = String::from_utf8_lossy(&entry.path).to_string();
                    let dest = workdir.join(&rel);
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&dest, blob.content());
                }
            }
        }

        // Stage every resolved conflicted file
        for file_desc in &session.conflicting_files {
            index
                .add_path(Path::new(&file_desc.file_path))
                .map_err(|e| {
                    GitUploadError::Git(format!("Failed staging '{}': {e}", file_desc.file_path))
                })?;
        }

        if index.has_conflicts() {
            return Err(GitUploadError::Git(
                "Unresolved conflict entries remain in index after applying resolutions"
                    .to_string(),
            ));
        }

        index
            .write()
            .map_err(|e| GitUploadError::Git(format!("Failed writing Git index: {e}")))?;
        let tree_id = index
            .write_tree()
            .map_err(|e| GitUploadError::Git(format!("Failed writing merge tree: {e}")))?;
        let merge_tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(format!("Failed finding written merge tree: {e}")))?;

        let sig = repo
            .signature()
            .or_else(|_| git2::Signature::now("GitUpload Desktop", "gitupload@desktop.local"))
            .map_err(|e| GitUploadError::Git(format!("Signature error: {e}")))?;

        let msg = commit_message
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                format!(
                    "chore(merge): merge '{}' into '{}'",
                    session.branch_theirs, session.branch_ours
                )
            });

        let oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                &msg,
                &merge_tree,
                &[&ours_commit, &theirs_commit],
            )
            .map_err(|e| GitUploadError::Git(format!("Failed committing 2-parent merge: {e}")))?;

        let _ = repo.cleanup_state();
        Ok(oid.to_string())
    }
}
