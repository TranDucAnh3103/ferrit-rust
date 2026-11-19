#![forbid(unsafe_code)]
//! Fast `libgit2` revwalk collector extracting unindexed `(commit_oid, summary)` pairs for semantic indexing.

use git2::{Repository, Sort};
use gitupload_core::errors::{GitError, GitResult};
use std::collections::HashSet;
use std::path::Path;

/// Walks repository history cleanly to extract commit summaries not yet indexed in `commit_embeddings`.
pub struct CommitIndexer;

impl CommitIndexer {
    /// Returns up to `max_commits` `(oid_hex, summary)` entries from `HEAD` excluding `already_indexed`.
    pub fn collect_unindexed_commits(
        repo_path: &Path,
        already_indexed: &HashSet<String>,
        max_commits: usize,
    ) -> GitResult<Vec<(String, String)>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitError::RepoOpen(repo_path.display().to_string(), e.to_string()))?;

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| GitError::Engine(format!("Revwalk init failed: {e}")))?;
        let _ = revwalk.set_sorting(Sort::TIME);
        if revwalk.push_head().is_err() {
            return Ok(Vec::new());
        }

        let mut pending = Vec::new();
        for oid_res in revwalk.take(max_commits.max(1)) {
            let Ok(oid) = oid_res else {
                continue;
            };
            let oid_str = oid.to_string();
            if already_indexed.contains(&oid_str) {
                continue;
            }
            if let Ok(commit) = repo.find_commit(oid) {
                let summary = commit.summary().unwrap_or("").trim().to_string();
                if !summary.is_empty() {
                    pending.push((oid_str, summary));
                }
            }
        }

        Ok(pending)
    }
}
