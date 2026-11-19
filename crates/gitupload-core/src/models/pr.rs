//! Pull Request (PR) lifecycle models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle state of a GitHub Pull Request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrState {
    /// Open pull request.
    Open,
    /// Closed without merge.
    Closed,
    /// Merged pull request.
    Merged,
    /// Query filter for all states.
    All,
}

/// Merge strategy supported when merging a GitHub Pull Request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeStrategy {
    /// Standard merge commit (`merge`).
    Merge,
    /// Squash commits into a single commit (`squash`).
    Squash,
    /// Rebase commits onto base branch (`rebase`).
    Rebase,
}

impl MergeStrategy {
    /// Returns the GitHub API `merge_method` string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

/// Pull Request metadata returned from GitHub API or local cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestInfo {
    /// GitHub numeric ID.
    pub id: u64,
    /// Repository-scoped PR number (`#42`).
    pub number: u64,
    /// PR title.
    pub title: String,
    /// Optional Markdown body.
    pub body: String,
    /// Current state (`"open"`, `"closed"`, `"merged"`).
    pub state: String,
    /// True if PR is marked as draft.
    pub is_draft: bool,
    /// Web URL for viewing the PR on GitHub.
    pub html_url: String,
    /// Head branch name.
    pub head_branch: String,
    /// Base target branch name.
    pub base_branch: String,
    /// Author GitHub username.
    pub author: String,
    /// Mergeability status when known.
    pub mergeable: Option<bool>,
    /// Number of conversation/review comments.
    pub comments_count: u32,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Request payload to create a Pull Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePullRequest {
    /// PR title.
    pub title: String,
    /// Head branch (or `user:branch` for cross-fork PRs).
    pub head: String,
    /// Base branch (e.g. `"main"`).
    pub base: String,
    /// Optional PR Markdown body.
    pub body: Option<String>,
    /// Create as Draft PR.
    pub draft: bool,
}

/// Request payload to merge a Pull Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergePullRequest {
    /// Optional custom merge commit title.
    pub commit_title: Option<String>,
    /// Optional custom merge commit message.
    pub commit_message: Option<String>,
    /// Merge strategy (`merge`, `squash`, `rebase`).
    pub merge_method: MergeStrategy,
}
