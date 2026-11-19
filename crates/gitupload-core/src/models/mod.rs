//! Domain data models and Serde contracts shared across all 10 crates.

pub mod analytics;
pub mod branch_cleaner;
pub mod conflict;
pub mod github;
pub mod graph;
pub mod org;
pub mod pr;
pub mod push;
pub mod remote;
pub mod security;
pub mod settings;
pub mod ssh;
pub mod sync;

pub use analytics::{
    ClassroomAssignment, DailyCommitStat, HeatmapDay, LeaderboardEntry, SubmissionEvaluation,
    SubmissionStatus, VelocityMetrics,
};
pub use branch_cleaner::{
    BranchMergeStatus, PruneBranchesRequest, PruneBranchesResult, StaleBranchInfo,
};
pub use conflict::{
    ConflictHunk, ConflictResolutionChoice, FileConflictDescriptor, FileResolutionRequest,
    MergeConflictSession,
};
pub use github::{
    ActionWorkflowRun, BranchProtectionRule, ContentInfo, FileContentInfo, GhCliStatus,
    GitHubProfile, IssueItem, ReleaseItem, RepositoryInfo, TokenReport,
};
pub use graph::{
    BranchSwimlane, GitCommitNode, GitDagResponse, GitGraphNode, GitRefBadge, GraphEdge,
    GraphLineSegment, GraphLineType, RefBadgeType,
};
pub use org::{AccountProfile, GitHubOrg, OrgMembershipRole};
pub use pr::{CreatePullRequest, MergePullRequest, MergeStrategy, PrState, PullRequestInfo};
pub use push::{PushHistoryRecord, PushMode, PushPayload, PushResult, RepoInspection, ResetMode};
pub use remote::{
    MultiRemoteProgressPayload, RemoteEndpoint, RemoteHealthReport, RemoteHealthStatus,
    RemoteProviderType, RemotePushResult, SyncPolicy, SyncStatus,
};
pub use security::{
    SecurityAuditReport, SecurityPatchProposal, VulnerabilityFinding, VulnerabilitySeverity,
};
pub use settings::{AppSettings, NamingPattern};
pub use ssh::{SshKeyAlgorithm, SshKeyDescriptor};
pub use sync::{AutoSyncOutcome, PowerState, SyncScheduleConfig};
