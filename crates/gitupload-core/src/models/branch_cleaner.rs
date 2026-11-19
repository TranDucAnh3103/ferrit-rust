//! Repository hygiene, stale branch categorization, and safe bulk pruning request/result models.

use serde::{Deserialize, Serialize};

/// Merge & staleness classification for a Git branch relative to the repository default branch.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum BranchMergeStatus {
    /// Fast-forward or merge commit is fully reachable from the default branch.
    FullyMerged,
    /// Pull request for this branch was squash-merged via GitHub API.
    SquashedAndMerged,
    /// Branch contains unmerged commits modified within the active window.
    UnmergedAhead,
    /// Branch has had no new commits for 90+ days and remains unmerged.
    StaleInactive,
}

/// Inspection descriptor for a single branch candidate during hygiene analysis.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct StaleBranchInfo {
    /// Short local or remote branch name (`feature/auth`).
    pub name: String,
    /// True for remote tracking refs.
    pub is_remote: bool,
    /// Optional remote alias (`origin`).
    pub remote_name: Option<String>,
    /// Tip commit SHA hex.
    pub last_commit_sha: String,
    /// Tip commit Unix timestamp in seconds.
    pub last_commit_date: i64,
    /// Age in days since tip commit.
    pub days_since_last_commit: u32,
    /// Last commit author name.
    pub author_name: String,
    /// Topological / GitHub merge classification.
    pub merge_status: BranchMergeStatus,
    /// Protected guardrail status (`main`, `master`, `develop`, `production`, `release/*`).
    pub is_protected: bool,
    /// Associated GitHub PR number if squash-merged.
    pub associated_pr_number: Option<u64>,
}

/// Request payload submitting selected branches for deletion & remote ref pruning.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PruneBranchesRequest {
    /// Target local repository path.
    pub repo_path: String,
    /// Branches selected for deletion.
    pub branch_names: Vec<String>,
    /// Whether to execute `git push origin --delete <branch>` as well.
    pub delete_remote: bool,
    /// Whether to execute `git remote prune origin`.
    pub prune_tracking_refs: bool,
}

/// Outcome summary of a bulk branch prune execution.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PruneBranchesResult {
    /// Count of local branches deleted.
    pub deleted_local_count: usize,
    /// Count of remote branches deleted.
    pub deleted_remote_count: usize,
    /// Failed or guardrail-blocked branches as `(branch_name, reason)`.
    pub failed_deletions: Vec<(String, String)>,
}
