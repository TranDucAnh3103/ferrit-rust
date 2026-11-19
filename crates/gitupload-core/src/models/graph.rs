//! Interactive Git Graph (DAG) and swimlane visualization data models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lightweight commit node produced by topological revwalk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitCommitNode {
    /// Full 40-character commit OID.
    pub oid: String,
    /// 7-character abbreviated SHA.
    pub short_oid: String,
    /// Full commit message body.
    pub message: String,
    /// First line summary of commit message.
    pub summary: String,
    /// Author display name.
    pub author_name: String,
    /// Author email address.
    pub author_email: String,
    /// Commit timestamp in UTC.
    pub timestamp: DateTime<Utc>,
    /// Parent commit OIDs (2+ indicates a merge commit).
    pub parent_oids: Vec<String>,
    /// Visual swimlane column index (`0, 1, 2...`).
    pub swimlane_index: usize,
    /// True if this commit is pointed to by `HEAD`.
    pub is_head: bool,
    /// Attached ref labels (branches, tags, stashes).
    pub refs: Vec<String>,
    /// True if GPG/SSH commit signature is present/verified.
    pub is_verified: bool,
}

/// Visual edge segment connecting parent and child commits in the DAG canvas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphLineSegment {
    /// Starting swimlane column index.
    pub from_col: usize,
    /// Target parent swimlane column index.
    pub to_col: usize,
    /// Row index of child node.
    pub from_row: usize,
    /// Row index of parent node.
    pub to_row: usize,
    /// Geometry type for Bezier curve routing.
    pub line_type: GraphLineType,
    /// Primer hex color associated with this branch lane.
    pub color: String,
}

/// Alias matching Phase 1 specification naming (`GraphEdge`).
pub type GraphEdge = GraphLineSegment;

/// Visual line connection type between commit rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphLineType {
    /// Direct vertical connection within the same swimlane.
    Straight,
    /// Branch-out Bezier curve to another swimlane.
    BranchOut,
    /// Merge-in Bezier curve from another swimlane.
    MergeIn,
}

/// Classification of a Git reference badge attached to a commit node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefBadgeType {
    /// Local branch (`refs/heads/*`).
    LocalBranch,
    /// Remote tracking branch (`refs/remotes/*`).
    RemoteBranch,
    /// Annotated or lightweight tag (`refs/tags/*`).
    Tag,
    /// Symbolic `HEAD` pointer.
    Head,
    /// Stash entry (`refs/stash`).
    Stash,
}

/// Reference badge rendered next to a commit summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitRefBadge {
    /// Display name of the ref (e.g. `main`, `origin/main`, `v2.2.0`).
    pub name: String,
    /// Classification for color/icon styling.
    pub ref_type: RefBadgeType,
    /// True if `HEAD` points directly or symbolically to this ref.
    pub is_head: bool,
}

/// Alias matching Phase 1 specification naming (`CommitRef`).
pub type CommitRef = GitRefBadge;

/// Color-coded branch swimlane descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchSwimlane {
    /// Swimlane column index.
    pub index: usize,
    /// Associated branch or ref name if known.
    pub branch_name: Option<String>,
    /// Primer palette hex color.
    pub color: String,
    /// Whether this lane is still active at the bottom of the current slice.
    pub is_active: bool,
}

/// Rich DAG commit node payload consumed by the virtualized 60 FPS Canvas UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitGraphNode {
    /// Full 40-char OID.
    pub oid: String,
    /// Abbreviated 7-char SHA.
    pub short_sha: String,
    /// Parent commit OIDs.
    pub parent_oids: Vec<String>,
    /// Author name.
    pub author_name: String,
    /// Author email.
    pub author_email: String,
    /// Optional avatar URL.
    pub author_avatar_url: Option<String>,
    /// Unix timestamp in seconds.
    pub timestamp: i64,
    /// First-line commit summary.
    pub summary: String,
    /// Optional multi-line commit body.
    pub body: Option<String>,
    /// Assigned swimlane column.
    pub column: usize,
    /// Primer hex color for this node.
    pub color: String,
    /// Outgoing visual edges to parents.
    pub lines: Vec<GraphLineSegment>,
    /// Attached reference badges.
    pub refs: Vec<GitRefBadge>,
    /// True when node represents a stash entry.
    pub is_stash: bool,
    /// True when commit has two or more parents.
    pub is_merge: bool,
    /// True when commit carries a cryptographic signature.
    pub signature_verified: bool,
}

/// Complete DAG response returned to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitDagResponse {
    /// Topologically sorted commit nodes with swimlane coordinates.
    pub nodes: Vec<GitGraphNode>,
    /// Swimlane lane metadata.
    pub swimlanes: Vec<BranchSwimlane>,
    /// Total number of nodes returned.
    pub total_commits: usize,
}
