#![deny(unsafe_code)]
//! Dual-engine Git abstraction: in-process `libgit2`, Windows Job Object CLI fallback,
//! interactive DAG revwalk graph builder, safe sync (`--force-with-lease`), Git LFS,
//! multi-remote mirroring, 3-way merge conflict resolution, commit analytics, DevSecOps patcher,
//! semantic indexer, stale branch cleaner, autonomous sync, and bundle backup.

pub mod analytics;
pub mod auto_sync;
pub mod backup;
pub mod cli;
pub mod coordinator;
pub mod engine;
pub mod gitignore;
pub mod graph;
pub mod indexer;
pub mod lfs;
pub mod libgit2;
pub mod merge;
pub mod mirror;
pub mod patcher;
pub mod pruner;
pub mod signing;
pub mod stale_analyzer;

pub use analytics::CommitAggregator;
pub use auto_sync::AutoSyncEngine;
pub use backup::GitBackupEngine;
pub use cli::{CliGitEngine, WindowsJobObject};
pub use coordinator::{
    AutoStashGuard, GitCoordinator, RePushInspector, SafePushMode, SafeSyncEngine,
};
pub use engine::{GitEngine, LibGit2GitEngine};
pub use gitignore::GitignoreGenerator;
pub use graph::{GitGraphActions, GitGraphEngine, SwimlaneRouter};
pub use indexer::CommitIndexer;
pub use lfs::{LfsCandidate, LfsManager};
pub use libgit2::{LibGit2Commit, LibGit2Repo, LibGit2Transport};
pub use merge::{CommitBuilder, ConflictParser, ConflictResolver, MergeCalculator};
pub use mirror::{MultiRemoteCoordinator, RemoteHealthChecker};
pub use patcher::AutomatedPatcher;
pub use pruner::BranchPruner;
pub use signing::CommitSigner;
pub use stale_analyzer::StaleBranchAnalyzer;
