//! Push workflow, history audit, and local repository inspection models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

fn default_safe_sync() -> bool {
    true
}

/// Strategy used to resolve the target repository name during push.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PushMode {
    /// Formatted using session & exercise numbers (e.g. `session15-ex02-homework`).
    SessionEx,
    /// Formatted as a mini-project repository (`session15-miniproject-todo`).
    MiniProject,
    /// Explicit user-supplied repository name.
    CustomRepository,
}

/// Request payload dispatched from the UI to start a Git push workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPayload {
    /// Local folder or single file path selected by the user.
    pub folder_path: String,
    /// Naming strategy for the destination repository.
    pub mode: PushMode,
    /// Session number string (for `SessionEx` / `MiniProject`).
    pub session_val: Option<String>,
    /// Exercise number string (for `SessionEx`).
    pub ex_val: Option<String>,
    /// Mini-project suffix (for `MiniProject`).
    pub mini_project_val: Option<String>,
    /// Explicit custom repository name (for `CustomRepository`).
    pub custom_repo_name: Option<String>,
    /// Whether to use configured default commit message/pattern.
    pub use_default_commit: bool,
    /// Custom commit message when `use_default_commit` is false.
    pub custom_commit_msg: Option<String>,
    /// True when uploading an isolated single file instead of a directory.
    pub is_file: bool,
    /// Target GitHub organization login (if pushing to an organization).
    pub target_org: Option<String>,
    /// Enforce safe sync (`--force-with-lease` & divergence check) rather than blind force.
    #[serde(default = "default_safe_sync")]
    pub safe_sync: bool,
}

/// Summary returned upon successful repository push.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushResult {
    /// Repository name on GitHub.
    pub repo_name: String,
    /// Full HTML URL of the repository.
    pub repo_url: String,
    /// Resulting commit SHA (40-hex OID).
    pub commit_sha: String,
    /// Target branch pushed (e.g. `"main"`).
    pub branch: String,
    /// Number of files included in the upload.
    pub files_count: u64,
    /// Total execution time in milliseconds.
    pub duration_ms: u64,
    /// Timestamp when the push completed.
    pub pushed_at: DateTime<Utc>,
}

/// Persistent audit entry stored in SQLite `push_history`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PushHistoryRecord {
    /// Unique record identifier.
    pub id: String,
    /// Account identifier associated with the push.
    pub account_id: String,
    /// Target repository name.
    pub repo_name: String,
    /// Remote repository URL (sanitized, never containing tokens).
    pub repo_url: String,
    /// Normalized local filesystem path.
    pub local_path: String,
    /// Commit SHA created during the push.
    pub commit_sha: Option<String>,
    /// Commit message used.
    pub commit_msg: String,
    /// Number of files touched.
    pub files_count: u64,
    /// Status string (`SUCCESS`, `FAILED`, `CANCELLED`).
    pub status: String,
    /// Error summary if push failed.
    pub error_message: Option<String>,
    /// Whether `--force-with-lease` was utilized.
    pub is_force_with_lease: bool,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
    /// ISO-8601 creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// Inspection snapshot of an existing local directory for Re-Push & Safe Sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoInspection {
    /// True if the directory already contains a valid `.git` repository.
    pub is_git_repo: bool,
    /// Currently checked-out branch name (or `"HEAD"` if detached).
    pub current_branch: Option<String>,
    /// Sanitized `origin` remote URL, if configured.
    pub remote_url: Option<String>,
    /// Number of local commits ahead of upstream tracking branch.
    pub ahead: usize,
    /// Number of upstream commits behind.
    pub behind: usize,
    /// List of modified/untracked/staged paths.
    pub dirty_files: Vec<String>,
    /// True when working tree or index has uncommitted changes.
    pub has_uncommitted_changes: bool,
    /// True when `.gitattributes` configures Git LFS filters.
    pub has_lfs: bool,
    /// True when native `.git/hooks` or `.gitupload/hooks.toml` are active.
    pub has_hooks: bool,
}

/// Supported modes for `git reset`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResetMode {
    /// Moves HEAD only, keeping index and working directory.
    Soft,
    /// Moves HEAD and resets index, keeping working directory.
    Mixed,
    /// Moves HEAD, resets index and working directory.
    Hard,
}
