#![forbid(unsafe_code)]
//! Fast `libgit2` commit revwalk aggregator collecting daily commit counts and line churn stats.

use chrono::{DateTime, Utc};
use git2::{DiffOptions, Repository, Sort};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::DailyCommitStat;
use std::collections::HashMap;
use std::path::Path;

/// Scans local Git commit DAG history into aggregated [`DailyCommitStat`] rows ready for SQLite.
pub struct CommitAggregator;

impl CommitAggregator {
    /// Walks up to `max_commits` reachable from `HEAD` and aggregates commits + line diff churn
    /// grouped by `(YYYY-MM-DD UTC, repo_name, author_email)`.
    pub fn scan_repository(
        repo_path: &Path,
        max_commits: usize,
    ) -> GitUploadResult<Vec<DailyCommitStat>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;

        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "repository".to_string());

        let mut revwalk = repo
            .revwalk()
            .map_err(|e| GitUploadError::Git(format!("Failed initializing revwalk: {e}")))?;
        if revwalk.push_head().is_err() {
            return Ok(Vec::new());
        }
        let _ = revwalk.set_sorting(Sort::TIME);

        let mut grouped: HashMap<(String, String), (u32, u64, u64)> = HashMap::new();
        let limit = max_commits.max(1);

        for oid_res in revwalk.take(limit) {
            let Ok(oid) = oid_res else {
                continue;
            };
            let Ok(commit) = repo.find_commit(oid) else {
                continue;
            };

            let ts_secs = commit.time().seconds();
            let dt = DateTime::<Utc>::from_timestamp(ts_secs, 0).unwrap_or_else(Utc::now);
            let date_key = dt.format("%Y-%m-%d").to_string();
            let author_email = commit
                .author()
                .email()
                .unwrap_or("unknown@local")
                .trim()
                .to_ascii_lowercase();

            let (added, deleted) = compute_commit_churn(&repo, &commit);
            let entry = grouped.entry((date_key, author_email)).or_insert((0, 0, 0));
            entry.0 = entry.0.saturating_add(1);
            entry.1 = entry.1.saturating_add(added);
            entry.2 = entry.2.saturating_add(deleted);
        }

        let mut stats: Vec<DailyCommitStat> = grouped
            .into_iter()
            .map(
                |((date, author_email), (commit_count, lines_added, lines_deleted))| {
                    DailyCommitStat {
                        date,
                        repo_name: repo_name.clone(),
                        commit_count,
                        lines_added,
                        lines_deleted,
                        author_email,
                    }
                },
            )
            .collect();

        stats.sort_by(|a, b| {
            a.date
                .cmp(&b.date)
                .then_with(|| a.author_email.cmp(&b.author_email))
        });
        Ok(stats)
    }
}

fn compute_commit_churn(repo: &Repository, commit: &git2::Commit<'_>) -> (u64, u64) {
    let Ok(curr_tree) = commit.tree() else {
        return (0, 0);
    };
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
    let mut opts = DiffOptions::new();
    let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&curr_tree), Some(&mut opts))
    else {
        return (0, 0);
    };
    let Ok(stats) = diff.stats() else {
        return (0, 0);
    };
    (stats.insertions() as u64, stats.deletions() as u64)
}
