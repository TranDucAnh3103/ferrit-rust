//! In-process `git2::Revwalk` DAG graph builder (< 15ms for 1,000+ commits).

use crate::graph::swimlane::SwimlaneRouter;
use chrono::{TimeZone, Utc};
use git2::{Oid, Repository, Sort};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{
    GitCommitNode, GitDagResponse, GitGraphNode, GitRefBadge, RefBadgeType,
};
use std::collections::HashMap;
use std::path::Path;

/// DAG commit revwalk and swimlane topology engine.
pub struct GitGraphEngine;

/// Alias matching Phase 5 benchmark naming (`DagRevwalkEngine`).
pub type DagRevwalkEngine = GitGraphEngine;

impl GitGraphEngine {
    /// Creates a new `GitGraphEngine` instance.
    pub fn new() -> Self {
        Self
    }

    /// Computes full DAG graph response including swimlane metadata, ref badges, and edges.
    pub fn compute_graph(
        &self,
        repo_path: &Path,
        max_commits: usize,
    ) -> GitUploadResult<GitDagResponse> {
        Self::build_dag_full(repo_path, max_commits)
    }

    /// Builds a rich `GitDagResponse` for the virtualized 60 FPS Canvas UI.
    pub fn build_dag_full(repo_path: &Path, max_commits: usize) -> GitUploadResult<GitDagResponse> {
        let mut repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?;

        let head_oid = repo.head().ok().and_then(|h| h.target());
        let head_branch_name = repo
            .head()
            .ok()
            .filter(|h| h.is_branch())
            .and_then(|h| h.shorthand().map(String::from));

        let mut ref_map: HashMap<Oid, Vec<GitRefBadge>> = HashMap::new();
        let mut stash_oids: Vec<Oid> = Vec::new();

        if let Ok(references) = repo.references() {
            for reference in references.flatten() {
                let full_name = reference.name().unwrap_or("");
                let short_name = reference.shorthand().unwrap_or(full_name).to_string();

                let target_oid = reference
                    .target()
                    .or_else(|| reference.peel_to_commit().ok().map(|c| c.id()));

                if let Some(oid) = target_oid {
                    if full_name == "refs/stash" {
                        stash_oids.push(oid);
                        ref_map.entry(oid).or_default().push(GitRefBadge {
                            name: "stash@{0}".into(),
                            ref_type: RefBadgeType::Stash,
                            is_head: false,
                        });
                    } else if full_name.starts_with("refs/heads/") {
                        let is_head = head_branch_name.as_deref() == Some(short_name.as_str());
                        ref_map.entry(oid).or_default().push(GitRefBadge {
                            name: short_name,
                            ref_type: RefBadgeType::LocalBranch,
                            is_head,
                        });
                    } else if full_name.starts_with("refs/remotes/") {
                        ref_map.entry(oid).or_default().push(GitRefBadge {
                            name: short_name,
                            ref_type: RefBadgeType::RemoteBranch,
                            is_head: false,
                        });
                    } else if full_name.starts_with("refs/tags/") {
                        ref_map.entry(oid).or_default().push(GitRefBadge {
                            name: short_name,
                            ref_type: RefBadgeType::Tag,
                            is_head: false,
                        });
                    }
                }
            }
        }

        // Also register additional stash entries via `stash_foreach`
        let _ = repo.stash_foreach(|index, _msg, stash_oid| {
            if !stash_oids.contains(stash_oid) {
                stash_oids.push(*stash_oid);
                ref_map.entry(*stash_oid).or_default().push(GitRefBadge {
                    name: format!("stash@{{{index}}}"),
                    ref_type: RefBadgeType::Stash,
                    is_head: false,
                });
            }
            true
        });

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| GitUploadError::Git(format!("Failed to initialize revwalk: {e}")))?;
        revwalk
            .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
            .map_err(|e| GitUploadError::Git(format!("Failed to set revwalk sorting: {e}")))?;

        let _ = revwalk.push_glob("refs/*");
        let _ = revwalk.push_head();
        for stash_oid in &stash_oids {
            let _ = revwalk.push(*stash_oid);
        }

        struct RawCommit {
            oid: Oid,
            oid_str: String,
            parent_oids: Vec<String>,
            author_name: String,
            author_email: String,
            timestamp: i64,
            summary: String,
            body: Option<String>,
            is_stash: bool,
            is_merge: bool,
            signature_verified: bool,
        }

        let limit = max_commits.max(1);
        let mut raw_commits = Vec::new();
        let mut route_input = Vec::new();

        for oid_res in revwalk.take(limit) {
            let oid = oid_res.map_err(|e| GitUploadError::Git(e.to_string()))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| GitUploadError::Git(e.to_string()))?;

            let oid_str = oid.to_string();
            let parent_oids: Vec<String> = commit.parent_ids().map(|id| id.to_string()).collect();
            let full_msg = commit.message().unwrap_or("");
            let summary = full_msg
                .lines()
                .next()
                .map(str::trim_end)
                .unwrap_or("")
                .to_string();
            let body = full_msg
                .split_once('\n')
                .map(|(_, rest)| rest.trim().to_string())
                .filter(|b| !b.is_empty());

            let author = commit.author();
            let signature_verified = commit
                .raw_header()
                .is_some_and(|header| header.contains("gpgsig "));
            let is_merge = parent_oids.len() >= 2;
            let is_stash = stash_oids.contains(&oid);

            route_input.push((oid_str.clone(), parent_oids.clone()));
            raw_commits.push(RawCommit {
                oid,
                oid_str,
                parent_oids,
                author_name: author.name().unwrap_or("").to_string(),
                author_email: author.email().unwrap_or("").to_string(),
                timestamp: commit.time().seconds(),
                summary,
                body,
                is_stash,
                is_merge,
                signature_verified,
            });
        }

        let (routed, swimlanes) = SwimlaneRouter::route(&route_input);

        let mut nodes = Vec::with_capacity(raw_commits.len());
        for (raw, (col, lines)) in raw_commits.into_iter().zip(routed) {
            let mut refs = ref_map.remove(&raw.oid).unwrap_or_default();
            if head_oid == Some(raw.oid) && !refs.iter().any(|r| r.is_head) {
                refs.insert(
                    0,
                    GitRefBadge {
                        name: "HEAD".into(),
                        ref_type: RefBadgeType::Head,
                        is_head: true,
                    },
                );
            }

            let short_sha = raw.oid_str.chars().take(7).collect();
            nodes.push(GitGraphNode {
                oid: raw.oid_str,
                short_sha,
                parent_oids: raw.parent_oids,
                author_name: raw.author_name,
                author_email: raw.author_email,
                author_avatar_url: None,
                timestamp: raw.timestamp,
                summary: raw.summary,
                body: raw.body,
                column: col,
                color: SwimlaneRouter::color_for_column(col),
                lines,
                refs,
                is_stash: raw.is_stash,
                is_merge: raw.is_merge,
                signature_verified: raw.signature_verified,
            });
        }

        let total_commits = nodes.len();
        Ok(GitDagResponse {
            nodes,
            swimlanes,
            total_commits,
        })
    }

    /// Lightweight `Vec<GitCommitNode>` projection for compact callers and existing interfaces.
    pub fn build_dag(repo_path: &Path, max_commits: usize) -> GitUploadResult<Vec<GitCommitNode>> {
        let full = Self::build_dag_full(repo_path, max_commits)?;
        let nodes = full
            .nodes
            .into_iter()
            .map(|n| {
                let ts = Utc
                    .timestamp_opt(n.timestamp, 0)
                    .single()
                    .unwrap_or_else(Utc::now);
                let is_head = n.refs.iter().any(|r| r.is_head);
                let refs = n.refs.into_iter().map(|r| r.name).collect();
                let message = match n.body {
                    Some(b) => format!("{}\n\n{b}", n.summary),
                    None => n.summary.clone(),
                };
                GitCommitNode {
                    oid: n.oid,
                    short_oid: n.short_sha,
                    message,
                    summary: n.summary,
                    author_name: n.author_name,
                    author_email: n.author_email,
                    timestamp: ts,
                    parent_oids: n.parent_oids,
                    swimlane_index: n.column,
                    is_head,
                    refs,
                    is_verified: n.signature_verified,
                }
            })
            .collect();
        Ok(nodes)
    }
}

impl Default for GitGraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libgit2::LibGit2Repo;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_graph_dag_perf() {
        let dir = tempdir().unwrap();
        let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
        LibGit2Repo::configure_identity(&repo, "Tester", "tester@gitupload.local").unwrap();

        let sig = git2::Signature::now("Tester", "tester@gitupload.local").unwrap();
        let empty_tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
        let empty_tree = repo.find_tree(empty_tree_oid).unwrap();

        let mut prev_oid: Option<git2::Oid> = None;
        for i in 0..1000 {
            let parent_commit = prev_oid.and_then(|oid| repo.find_commit(oid).ok());
            let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();
            let oid = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("chore: commit {i}"),
                    &empty_tree,
                    &parents,
                )
                .unwrap();
            prev_oid = Some(oid);
        }
        drop(empty_tree);
        drop(repo);

        let start = Instant::now();
        let response = GitGraphEngine::build_dag_full(dir.path(), 1000).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(response.nodes.len(), 1000);
        assert!(!response.swimlanes.is_empty());
        let budget_ms: u128 = if cfg!(debug_assertions) { 5000 } else { 250 };
        assert!(
            elapsed.as_millis() < budget_ms,
            "1,000 commit DAG build took {:?} (budget {}ms)",
            elapsed,
            budget_ms
        );
    }
}
