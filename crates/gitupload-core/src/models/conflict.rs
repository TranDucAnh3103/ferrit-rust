//! 3-way merge conflict detection, hunk parsing, and resolution request models.

use serde::{Deserialize, Serialize};

/// Resolution action chosen for an individual conflict hunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "content", rename_all = "snake_case")]
pub enum ConflictResolutionChoice {
    /// Keep local branch (`HEAD` / Ours) lines.
    AcceptCurrent,
    /// Keep remote branch (Incoming / Theirs) lines.
    AcceptIncoming,
    /// Combine local lines followed by incoming lines.
    AcceptBoth,
    /// Apply custom user or Cloud AI reconciled text.
    Custom(String),
}

/// Structured representation of one conflict block within a conflicted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictHunk {
    /// Deterministic identifier (`<sha1_prefix>:<index>`).
    pub hunk_id: String,
    /// Relative path inside the repository.
    pub file_path: String,
    /// Zero-based hunk index inside the file.
    pub hunk_index: usize,
    /// 1-based start line of the conflict marker in the merged/conflicted buffer.
    pub start_line_ours: usize,
    /// 1-based end line of the ours segment.
    pub end_line_ours: usize,
    /// 1-based start line of the theirs segment.
    pub start_line_theirs: usize,
    /// 1-based end line of the closing `>>>>>>>` marker.
    pub end_line_theirs: usize,
    /// Content lines from `HEAD` (`Ours`).
    pub content_ours: String,
    /// Content lines from incoming branch (`Theirs`).
    pub content_theirs: String,
    /// Optional content lines from common ancestor (`Base`).
    pub content_base: Option<String>,
}

/// Complete conflict descriptor for a single file across Base / Ours / Theirs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileConflictDescriptor {
    /// Relative repository path.
    pub file_path: String,
    /// True if any involved blob contains binary NUL bytes.
    pub is_binary: bool,
    /// Total conflict hunks detected in this file.
    pub total_conflicts: usize,
    /// Full file text from local `HEAD` (`Ours`).
    pub full_ours: String,
    /// Full file text from incoming commit (`Theirs`).
    pub full_theirs: String,
    /// Full file text from merge base (`Base`), if present.
    pub full_base: Option<String>,
    /// Synthesized conflict-marked text (`<<<<<<<`, `=======`, `>>>>>>>`) for parsing & fallback display.
    pub raw_conflicted_text: String,
    /// Ordered conflict hunks extracted from `raw_conflicted_text`.
    pub hunks: Vec<ConflictHunk>,
}

/// Active 3-way merge conflict session returned to the visual resolver.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeConflictSession {
    /// Repository root directory path.
    pub repo_path: String,
    /// Local branch or ref name (`HEAD`).
    pub branch_ours: String,
    /// Incoming ref/branch name (`origin/main`, commit OID, etc.).
    pub branch_theirs: String,
    /// Local commit OID hex.
    pub ours_oid: String,
    /// Incoming commit OID hex.
    pub theirs_oid: String,
    /// Per-file conflict breakdowns.
    pub conflicting_files: Vec<FileConflictDescriptor>,
}

/// Resolution plan submitted for a single conflicted file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileResolutionRequest {
    /// Relative file path matching `FileConflictDescriptor::file_path`.
    pub file_path: String,
    /// Mapping of `(hunk_index, resolution_choice)`.
    pub chunk_resolutions: Vec<(usize, ConflictResolutionChoice)>,
    /// Optional full-file manual override buffer.
    pub custom_content: Option<String>,
}
