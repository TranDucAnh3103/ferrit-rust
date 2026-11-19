#![forbid(unsafe_code)]
//! Non-destructive in-memory 3-way merge conflict detection using `libgit2::Repository::merge_trees`.

use crate::merge::parser::ConflictParser;
use git2::{IndexEntry, Repository};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{FileConflictDescriptor, MergeConflictSession};
use std::path::Path;

/// Calculates 3-way merge conflict descriptors without modifying the working tree unless finalized.
pub struct MergeCalculator;

impl MergeCalculator {
    /// Inspects `HEAD` vs `incoming_ref` (e.g. `"origin/main"` or a commit OID) in memory and returns all conflicts.
    pub fn detect_conflicts(
        repo_path: &Path,
        incoming_ref: &str,
    ) -> GitUploadResult<MergeConflictSession> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;

        let head_ref = repo
            .head()
            .map_err(|e| GitUploadError::Git(format!("Repository has no valid HEAD: {e}")))?;
        let branch_ours = head_ref
            .shorthand()
            .map(str::to_string)
            .unwrap_or_else(|| "HEAD".to_string());
        let ours_commit = head_ref
            .peel_to_commit()
            .map_err(|e| GitUploadError::Git(format!("HEAD commit lookup failed: {e}")))?;

        let theirs_obj = repo.revparse_single(incoming_ref.trim()).map_err(|e| {
            GitUploadError::Git(format!("Incoming ref '{incoming_ref}' not found: {e}"))
        })?;
        let theirs_commit = theirs_obj.peel_to_commit().map_err(|e| {
            GitUploadError::Git(format!(
                "Incoming target '{incoming_ref}' is not a commit: {e}"
            ))
        })?;

        let base_tree = match repo
            .merge_base(ours_commit.id(), theirs_commit.id())
            .ok()
            .and_then(|base_id| repo.find_commit(base_id).ok())
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

        let mut conflicting_files: Vec<FileConflictDescriptor> = Vec::new();
        if merged_index.has_conflicts() {
            let conflicts = merged_index
                .conflicts()
                .map_err(|e| GitUploadError::Git(format!("Failed reading index conflicts: {e}")))?;

            for conflict_res in conflicts {
                let conflict = conflict_res
                    .map_err(|e| GitUploadError::Git(format!("Corrupt conflict entry: {e}")))?;

                let file_path = extract_conflict_path(
                    conflict.our.as_ref(),
                    conflict.their.as_ref(),
                    conflict.ancestor.as_ref(),
                );

                let base_bytes = read_entry_bytes(&repo, conflict.ancestor.as_ref())?;
                let ours_bytes = read_entry_bytes(&repo, conflict.our.as_ref())?;
                let theirs_bytes = read_entry_bytes(&repo, conflict.their.as_ref())?;

                let is_binary = base_bytes.as_ref().is_some_and(|b| b.contains(&0))
                    || ours_bytes.as_ref().is_some_and(|b| b.contains(&0))
                    || theirs_bytes.as_ref().is_some_and(|b| b.contains(&0));

                if is_binary {
                    conflicting_files.push(FileConflictDescriptor {
                        file_path,
                        is_binary: true,
                        total_conflicts: 1,
                        full_ours: "[Binary Content]".to_string(),
                        full_theirs: "[Binary Content]".to_string(),
                        full_base: Some("[Binary Content]".to_string()),
                        raw_conflicted_text: String::new(),
                        hunks: Vec::new(),
                    });
                    continue;
                }

                let full_base = base_bytes.map(|b| String::from_utf8_lossy(&b).to_string());
                let full_ours = ours_bytes
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default();
                let full_theirs = theirs_bytes
                    .map(|b| String::from_utf8_lossy(&b).to_string())
                    .unwrap_or_default();

                let raw_conflicted_text = synthesize_three_way_markers(
                    full_base.as_deref(),
                    &full_ours,
                    &full_theirs,
                    &branch_ours,
                    incoming_ref.trim(),
                );
                let hunks = ConflictParser::parse(&file_path, &raw_conflicted_text);
                let total_conflicts = hunks.len();

                conflicting_files.push(FileConflictDescriptor {
                    file_path,
                    is_binary: false,
                    total_conflicts,
                    full_ours,
                    full_theirs,
                    full_base,
                    raw_conflicted_text,
                    hunks,
                });
            }
        }

        Ok(MergeConflictSession {
            repo_path: repo_path.to_string_lossy().to_string(),
            branch_ours,
            branch_theirs: incoming_ref.trim().to_string(),
            ours_oid: ours_commit.id().to_string(),
            theirs_oid: theirs_commit.id().to_string(),
            conflicting_files,
        })
    }
}

/// Synthesizes deterministic `diff3` conflict markers preserving shared prefix/suffix context lines.
fn synthesize_three_way_markers(
    base_opt: Option<&str>,
    ours_raw: &str,
    theirs_raw: &str,
    branch_ours: &str,
    branch_theirs: &str,
) -> String {
    let ours_norm = ours_raw.replace("\r\n", "\n");
    let theirs_norm = theirs_raw.replace("\r\n", "\n");
    let base_norm = base_opt.map(|s| s.replace("\r\n", "\n"));

    let ours_lines: Vec<&str> = ours_norm.split('\n').collect();
    let theirs_lines: Vec<&str> = theirs_norm.split('\n').collect();
    let base_lines: Option<Vec<&str>> = base_norm.as_deref().map(|s| s.split('\n').collect());

    let mut prefix_len = 0usize;
    let max_prefix = ours_lines
        .len()
        .min(theirs_lines.len())
        .min(base_lines.as_ref().map_or(usize::MAX, Vec::len));
    while prefix_len < max_prefix && ours_lines[prefix_len] == theirs_lines[prefix_len] {
        if let Some(ref b) = base_lines {
            if b[prefix_len] != ours_lines[prefix_len] {
                break;
            }
        }
        prefix_len += 1;
    }

    let mut suffix_len = 0usize;
    let ours_rem = ours_lines.len().saturating_sub(prefix_len);
    let theirs_rem = theirs_lines.len().saturating_sub(prefix_len);
    let base_rem = base_lines
        .as_ref()
        .map_or(usize::MAX, |b| b.len().saturating_sub(prefix_len));
    let max_suffix = ours_rem.min(theirs_rem).min(base_rem);
    while suffix_len < max_suffix {
        let o = ours_lines[ours_lines.len() - 1 - suffix_len];
        let t = theirs_lines[theirs_lines.len() - 1 - suffix_len];
        if o != t {
            break;
        }
        if let Some(ref b) = base_lines {
            if b[b.len() - 1 - suffix_len] != o {
                break;
            }
        }
        suffix_len += 1;
    }

    let mut out: Vec<String> = Vec::with_capacity(ours_lines.len() + theirs_lines.len() + 8);
    for line in &ours_lines[..prefix_len] {
        out.push((*line).to_string());
    }

    let ours_mid = &ours_lines[prefix_len..ours_lines.len() - suffix_len];
    let theirs_mid = &theirs_lines[prefix_len..theirs_lines.len() - suffix_len];

    if ours_mid == theirs_mid {
        for line in ours_mid {
            out.push((*line).to_string());
        }
    } else {
        out.push(format!("<<<<<<< {branch_ours}"));
        for line in ours_mid {
            out.push((*line).to_string());
        }
        if let Some(ref b) = base_lines {
            let base_mid = &b[prefix_len..b.len() - suffix_len];
            out.push("||||||| base".to_string());
            for line in base_mid {
                out.push((*line).to_string());
            }
        }
        out.push("=======".to_string());
        for line in theirs_mid {
            out.push((*line).to_string());
        }
        out.push(format!(">>>>>>> {branch_theirs}"));
    }

    for line in &ours_lines[ours_lines.len() - suffix_len..] {
        out.push((*line).to_string());
    }

    out.join("\n")
}

fn extract_conflict_path(
    our: Option<&IndexEntry>,
    their: Option<&IndexEntry>,
    ancestor: Option<&IndexEntry>,
) -> String {
    let raw = our
        .or(their)
        .or(ancestor)
        .map(|entry| entry.path.as_slice())
        .unwrap_or_default();
    String::from_utf8_lossy(raw).to_string()
}

fn read_entry_bytes(
    repo: &Repository,
    entry: Option<&IndexEntry>,
) -> GitUploadResult<Option<Vec<u8>>> {
    let Some(item) = entry else {
        return Ok(None);
    };
    let blob = repo
        .find_blob(item.id)
        .map_err(|e| GitUploadError::Git(format!("Blob lookup failed ({}): {e}", item.id)))?;
    Ok(Some(blob.content().to_vec()))
}
