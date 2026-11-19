//! GitHub ecosystem models: GhCliStatus, Actions, Issues, Releases, Branch Protection, and Repo info.

use serde::{Deserialize, Serialize};

/// Status of the local/sandboxed GitHub CLI (`gh`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GhCliStatus {
    /// True if `gh` binary is available either in sandbox or PATH.
    pub is_installed: bool,
    /// True if the resolved `gh` binary resides inside the application sandbox.
    pub is_sandboxed: bool,
    /// Detected semantic version string (e.g. `"2.62.0"`).
    pub version: Option<String>,
    /// Full filesystem path to the resolved executable.
    pub binary_path: Option<String>,
}

/// GitHub Actions CI/CD workflow run summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionWorkflowRun {
    /// Run ID.
    pub id: u64,
    /// Workflow name.
    pub name: String,
    /// Triggering branch name.
    pub head_branch: String,
    /// Commit SHA associated with the run.
    pub head_sha: String,
    /// Trigger event (`"push"`, `"pull_request"`, `"workflow_dispatch"`).
    pub event: String,
    /// Execution status (`"queued"`, `"in_progress"`, `"completed"`).
    pub status: String,
    /// Outcome conclusion (`"success"`, `"failure"`, `"cancelled"`).
    pub conclusion: Option<String>,
    /// Web URL to inspect the run on GitHub.
    pub html_url: String,
    /// Creation timestamp ISO-8601 string.
    pub created_at: String,
}

/// GitHub Issue item summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IssueItem {
    /// Numeric issue ID.
    pub id: u64,
    /// Repository issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Optional Markdown description.
    pub body: Option<String>,
    /// Issue state (`"open"`, `"closed"`).
    pub state: String,
    /// Author login.
    pub author: String,
    /// Attached label names.
    pub labels: Vec<String>,
    /// Creation timestamp ISO-8601 string.
    pub created_at: String,
}

/// GitHub Release item summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseItem {
    /// Numeric release ID.
    pub id: u64,
    /// Git tag name (e.g. `"v2.2.0"`).
    pub tag_name: String,
    /// Display title of the release.
    pub name: String,
    /// Markdown release notes / changelog body.
    pub body: Option<String>,
    /// True if release is a draft.
    pub draft: bool,
    /// True if release is marked as pre-release.
    pub prerelease: bool,
    /// Web URL of the release.
    pub html_url: String,
    /// Creation timestamp ISO-8601 string.
    pub created_at: String,
}

/// Branch protection policy snapshot returned by GitHub API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchProtectionRule {
    /// Branch name (e.g. `"main"`).
    pub branch: String,
    /// True if protection rules are enforced on the branch.
    pub protected: bool,
    /// True if rules also apply to repository admins.
    pub enforce_admins: bool,
    /// True if a Pull Request review is mandatory before merge.
    pub require_pr_reviews: bool,
    /// Minimum approving reviews required.
    pub required_approvals: u8,
    /// True if linear commit history is required.
    pub require_linear_history: bool,
    /// True if force pushes are allowed.
    pub allow_force_pushes: bool,
}

/// Repository metadata returned from GitHub REST API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositoryInfo {
    /// Repository short name.
    pub name: String,
    /// Full `owner/repo` name if provided.
    #[serde(default)]
    pub full_name: String,
    /// Web URL.
    pub html_url: String,
    /// HTTPS clone URL.
    #[serde(default)]
    pub clone_url: String,
    /// Repository size in KB (0 indicates an empty repository).
    #[serde(default)]
    pub size: u64,
    /// Optional repository description.
    pub description: Option<String>,
    /// Stargazer count.
    #[serde(default)]
    pub stargazers_count: u64,
    /// Fork count.
    #[serde(default)]
    pub forks_count: u64,
    /// Watcher count.
    #[serde(default)]
    pub watchers_count: u64,
    /// Private visibility flag.
    #[serde(default)]
    pub private: bool,
    /// Creation ISO timestamp.
    #[serde(default)]
    pub created_at: String,
    /// Last update ISO timestamp.
    #[serde(default)]
    pub updated_at: String,
    /// Last push ISO timestamp.
    pub pushed_at: Option<String>,
    /// Default branch name (empty if repository is empty).
    pub default_branch: Option<String>,
    /// Primary programming language if detected.
    pub language: Option<String>,
}

impl RepositoryInfo {
    /// Returns true if the remote repository contains zero commits / bytes.
    pub fn is_empty(&self) -> bool {
        self.size == 0 || self.default_branch.as_deref().unwrap_or("").is_empty()
    }
}

/// Entry inside a remote repository directory tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentInfo {
    /// File or folder name.
    pub name: String,
    /// Relative path inside repository.
    pub path: String,
    /// Item type (`"file"` or `"dir"`).
    #[serde(rename = "type")]
    pub item_type: String,
    /// Size in bytes.
    #[serde(default)]
    pub size: u64,
    /// Optional direct download URL.
    pub download_url: Option<String>,
}

/// File contents payload returned by GitHub Contents API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileContentInfo {
    /// File name.
    pub name: String,
    /// Path inside repository.
    pub path: String,
    /// Blob SHA required for atomic file updates.
    pub sha: String,
    /// File size in bytes.
    pub size: u64,
    /// Base64-encoded content.
    pub content: String,
    /// Encoding (`"base64"`).
    pub encoding: String,
}

/// Authenticated GitHub user profile summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitHubProfile {
    /// GitHub login handle.
    #[serde(default)]
    pub login: String,
    /// Avatar image URL.
    #[serde(default)]
    pub avatar_url: String,
    /// Display name.
    pub name: Option<String>,
    /// Bio text.
    pub bio: Option<String>,
    /// Followers count.
    #[serde(default)]
    pub followers: u64,
    /// Public repositories count.
    #[serde(default)]
    pub public_repos: u64,
    /// Private repositories count.
    #[serde(default)]
    pub total_private_repos: u64,
}

/// Comprehensive diagnostic report for the active GitHub token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenReport {
    /// OAuth scopes granted to the token.
    pub scopes: Vec<String>,
    /// Hourly API rate limit.
    pub rate_limit: u64,
    /// Remaining API requests in the current window.
    pub rate_remaining: u64,
    /// Formatted reset time (`HH:MM:SS`).
    pub rate_reset: String,
    /// Public repository count.
    pub public_repos: u64,
    /// Private repository count.
    pub private_repos: u64,
    /// Disk usage in KB.
    pub disk_usage: u64,
    /// GitHub subscription plan name.
    pub plan_name: String,
    /// Two-factor authentication enabled status.
    pub two_factor_authentication: bool,
    /// True if `repo` scope is present.
    pub has_repo_scope: bool,
    /// True if `delete_repo` scope is present.
    pub has_delete_scope: bool,
    /// True if `admin:org` or `write:org` scope is present.
    pub has_org_scope: bool,
    /// GitHub username.
    pub username: String,
    /// Avatar URL.
    pub avatar_url: String,
    /// Primary email if public.
    pub email: Option<String>,
}
