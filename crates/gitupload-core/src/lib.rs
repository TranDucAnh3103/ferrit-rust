#![forbid(unsafe_code)]
//! Core domain models, shared error hierarchy, repository naming utilities, and batch scheduling.

pub mod airgap;
pub mod errors;
pub mod marketplace;
pub mod models;
pub mod naming;
pub mod plugin_manifest;
pub mod policy_file;
pub mod scheduler;
pub mod vector_search;
pub mod webhooks;

pub use airgap::AirGapGuard;
pub use errors::{
    AiError, AiResult, AppError, AppResult, GhCliError, GhCliResult, GitError, GitHubError,
    GitHubResult, GitResult, GitUploadError, GitUploadResult, HookError, HookResult,
    IpcErrorPayload, StorageError, StorageResult,
};
pub use marketplace::*;
pub use models::*;
pub use naming::{
    format_repo_name, parse_file_metadata, resolve_commit_msg_for_file, resolve_repo_name,
    sanitize_mini_project_text, slugify_repo_name, validate_repository_name, FileHomeworkMetadata,
};
pub use plugin_manifest::*;
pub use policy_file::{EnterprisePolicyFile, EnterpriseSection};
pub use scheduler::BatchScheduler;
pub use vector_search::{SearchHit, VectorSearchEngine};
pub use webhooks::*;
