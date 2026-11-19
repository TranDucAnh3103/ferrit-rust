#![forbid(unsafe_code)]
//! `libgit2` topological merge-base analyzer categorizing local branches for safe stale cleanup.

use chrono::Utc;
use git2::{BranchType, Repository};
use gitupload_core::errors::{GitError, GitResult};
use gitupload_core::models::{BranchMergeStatus, StaleBranchInfo};

/// Analyzes branch topology against the default branch (`main` / `master`) to detect merged or stale branches.
pub struct StaleBranchAnalyzer;

impl StaleBranchAnalyzer {
    /// Returns `true` if `name` matches mandatory protection rules (`main`, `master`, `develop`, `production`, `release/*`).
    pub fn is_protected_branch(name: &str) -> bool {
        let clean = name.trim().trim_start_matches("origin/");
        matches!(clean, "main" | "master" | "develop" | "production")
            || clean.starts_with("release/")
    }

    /// Scans all local branches relative to `default_branch_name` (or auto-detects `main`/`master`).
    pub fn analyze_local_branches(
        repo_path: &str,
        default_branch_name: &str,
    ) -> GitResult<Vec<StaleBranchInfo>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitError::RepoOpen(repo_path.to_string(), e.to_string()))?;

        let resolved_default = if repo
            .find_branch(default_branch_name, BranchType::Local)
            .is_ok()
        {
            default_branch_name.to_string()
        } else if repo.find_branch("main", BranchType::Local).is_ok() {
            "main".to_string()
        } else if repo.find_branch("master", BranchType::Local).is_ok() {
            "master".to_string()
        } else {
            return Err(GitError::Engine(format!(
                "Default branch '{default_branch_name}' (nor fallback main/master) not found"
            )));
        };

        let default_branch = repo
            .find_branch(&resolved_default, BranchType::Local)
            .map_err(|e| {
                GitError::Engine(format!(
                    "Default branch '{resolved_default}' not found: {e}"
                ))
            })?;
        let default_commit = default_branch
            .get()
            .peel_to_commit()
            .map_err(|e| GitError::Engine(e.to_string()))?;
        let default_oid = default_commit.id();

        let branches = repo
            .branches(Some(BranchType::Local))
            .map_err(|e| GitError::Engine(e.to_string()))?;

        let mut results = Vec::new();
        let now = Utc::now().timestamp();

        for branch_res in branches {
            let (branch, _) = branch_res.map_err(|e| GitError::Engine(e.to_string()))?;
            let name = branch
                .name()
                .map_err(|e| GitError::Engine(e.to_string()))?
                .unwrap_or("")
                .to_string();

            if name.is_empty() || name == resolved_default {
                continue;
            }

            let is_protected = Self::is_protected_branch(&name);
            let target_commit = branch
                .get()
                .peel_to_commit()
                .map_err(|e| GitError::Engine(e.to_string()))?;
            let target_oid = target_commit.id();

            let commit_time = target_commit.time().seconds();
            let days_ago = ((now - commit_time).max(0) / 86_400) as u32;

            let merge_base = repo.merge_base(default_oid, target_oid).ok();
            let merge_status = if merge_base == Some(target_oid) {
                BranchMergeStatus::FullyMerged
            } else if days_ago >= 90 {
                BranchMergeStatus::StaleInactive
            } else {
                BranchMergeStatus::UnmergedAhead
            };

            results.push(StaleBranchInfo {
                name,
                is_remote: false,
                remote_name: None,
                last_commit_sha: target_oid.to_string(),
                last_commit_date: commit_time,
                days_since_last_commit: days_ago,
                author_name: target_commit
                    .author()
                    .name()
                    .unwrap_or("Unknown")
                    .to_string(),
                merge_status,
                is_protected,
                associated_pr_number: None,
            });
        }

        Ok(results)
    }
}
