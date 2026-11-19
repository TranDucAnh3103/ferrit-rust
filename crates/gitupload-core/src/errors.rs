//! Centralized strongly-typed domain errors and IPC error mapping.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Unified result type used across workspace crates.
pub type GitUploadResult<T> = Result<T, GitUploadError>;

/// Alias for application-level result.
pub type AppResult<T> = GitUploadResult<T>;

/// Alias for application-level error.
pub type AppError = GitUploadError;

/// Specialized result type for GitHub CLI lifecycle operations.
pub type GhCliResult<T> = Result<T, GhCliError>;

/// Specialized result type for Cloud AI operations.
pub type AiResult<T> = Result<T, AiError>;

/// Specialized result type for GitHub REST/GraphQL API operations.
pub type GitHubResult<T> = Result<T, GitHubError>;

/// Specialized result type for Git engine operations.
pub type GitResult<T> = Result<T, GitError>;

/// Specialized result type for SQLite storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Specialized result type for native Git hooks operations.
pub type HookResult<T> = Result<T, HookError>;

/// Root domain error tree for GitUpload Desktop.
#[derive(Error, Debug)]
pub enum GitUploadError {
    /// Filesystem or OS I/O error.
    #[error("Filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration or settings state.
    #[error("Configuration error: {0}")]
    Config(String),

    /// OS Credential Vault / keyring failure.
    #[error("OS Credential Vault error: {0}")]
    Vault(String),

    /// Git engine operation error.
    #[error("Git engine error: {0}")]
    Git(String),

    /// Remote divergence intercepted during safe sync.
    #[error("Remote divergence detected (ahead {ahead}, behind {behind}): {message}")]
    DivergenceDetected {
        /// Local commits ahead of upstream.
        ahead: usize,
        /// Remote commits ahead of local.
        behind: usize,
        /// Descriptive explanation.
        message: String,
    },

    /// GitHub REST/GraphQL API failure.
    #[error("GitHub API error (status {status}): {message}")]
    GitHubApi {
        /// HTTP status code.
        status: u16,
        /// API error details.
        message: String,
    },

    /// Native Git hook validation failure.
    #[error("Native Hook validation failed: {0}")]
    Hook(String),

    /// GitHub CLI sandbox or auth bridge error.
    #[error("GitHub CLI sandbox error: {0}")]
    GhCli(String),

    /// Cloud AI service failure.
    #[error("Cloud AI service error: {0}")]
    CloudAi(String),

    /// SQLite database or migration error.
    #[error("Storage / SQLite error: {0}")]
    Storage(String),

    /// Pre-flight safety guard blocked a dangerous or invalid action.
    #[error("Pre-flight safety guard blocked push: {0}")]
    SafetyGuard(String),

    /// Cooperative cancellation requested by user or scheduler.
    #[error("Operation was cancelled by user")]
    Cancelled,

    /// Cooperative cancellation with contextual reason.
    #[error("Operation cancelled: {0}")]
    OperationCancelled(String),

    /// Unexpected internal failure.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Domain errors for `gitupload-ghcli`.
#[derive(Error, Debug)]
pub enum GhCliError {
    /// GitHub CLI is not installed in PATH or sandbox.
    #[error("GitHub CLI (gh) is not installed")]
    NotInstalled,

    /// Failed spawning or executing `gh` process.
    #[error("GitHub CLI process error: {0}")]
    Process(String),

    /// Filesystem I/O failure during sandbox management.
    #[error("GitHub CLI I/O error: {0}")]
    Io(String),

    /// Zip archive extraction failure.
    #[error("GitHub CLI archive error: {0}")]
    Archive(String),

    /// Cryptographic SHA256 verification failed.
    #[error("SHA256 checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Expected SHA256 digest hex.
        expected: String,
        /// Computed SHA256 digest hex.
        actual: String,
    },

    /// Network download failure.
    #[error("GitHub CLI download error: {0}")]
    Network(String),

    /// Dual Auth Bridge authentication failure.
    #[error("GitHub CLI auth bridge error: {0}")]
    Auth(String),
}

impl From<GhCliError> for GitUploadError {
    fn from(err: GhCliError) -> Self {
        Self::GhCli(err.to_string())
    }
}

/// Domain errors for `gitupload-ai`.
#[derive(Error, Debug)]
pub enum AiError {
    /// Network transport error.
    #[error("Cloud AI network error: {0}")]
    Network(String),

    /// Non-success HTTP response from Cloud AI provider.
    #[error("Cloud AI API error: {0}")]
    ApiError(String),

    /// Provider rate-limit or quota exhaustion.
    #[error("Cloud AI rate limited: {0}")]
    RateLimited(String),

    /// Missing provider API key in OS Keyring.
    #[error("Missing Cloud AI API key for provider: {0}")]
    MissingKey(String),

    /// Invalid or malformed response payload.
    #[error("Invalid Cloud AI response: {0}")]
    InvalidResponse(String),
}

impl From<AiError> for GitUploadError {
    fn from(err: AiError) -> Self {
        Self::CloudAi(err.to_string())
    }
}

/// Domain errors for `gitupload-github`.
#[derive(Error, Debug)]
pub enum GitHubError {
    /// Network or HTTP client failure.
    #[error("GitHub network error: {0}")]
    Network(String),

    /// Invalid or expired credentials (HTTP 401).
    #[error("GitHub unauthorized: {0}")]
    Unauthorized(String),

    /// Missing required OAuth scope or permission (HTTP 403).
    #[error("GitHub forbidden: {0}")]
    Forbidden(String),

    /// Resource not found (HTTP 404).
    #[error("GitHub resource not found: {0}")]
    NotFound(String),

    /// General GitHub API failure.
    #[error("GitHub API error (status {status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message returned by GitHub.
        message: String,
    },

    /// JSON response decoding failure.
    #[error("GitHub response parse error: {0}")]
    Parse(String),
}

impl From<GitHubError> for GitUploadError {
    fn from(err: GitHubError) -> Self {
        match err {
            GitHubError::Api { status, message } => Self::GitHubApi { status, message },
            GitHubError::Unauthorized(msg) => Self::GitHubApi {
                status: 401,
                message: msg,
            },
            GitHubError::Forbidden(msg) => Self::GitHubApi {
                status: 403,
                message: msg,
            },
            GitHubError::NotFound(msg) => Self::GitHubApi {
                status: 404,
                message: msg,
            },
            other => Self::GitHubApi {
                status: 0,
                message: other.to_string(),
            },
        }
    }
}

/// Domain errors for `gitupload-git`.
#[derive(Error, Debug)]
pub enum GitError {
    /// Repository open failure.
    #[error("Failed to open repository at '{0}': {1}")]
    RepoOpen(String, String),

    /// Merge or divergence conflict.
    #[error("Conflict detected: {0}")]
    Conflict(String),

    /// Operation timeout.
    #[error("Git operation timed out after {0} seconds")]
    Timeout(u64),

    /// Authentication failure.
    #[error("Git authentication error: {0}")]
    Auth(String),

    /// General Git engine error.
    #[error("{0}")]
    Engine(String),
}

impl From<GitError> for GitUploadError {
    fn from(err: GitError) -> Self {
        Self::Git(err.to_string())
    }
}

/// Domain errors for `gitupload-storage`.
#[derive(Error, Debug)]
pub enum StorageError {
    /// SQLite connection or query failure.
    #[error("SQLite query failed: {0}")]
    Sqlite(String),

    /// Legacy data migration error.
    #[error("Legacy migration failed: {0}")]
    Migration(String),
}

impl From<StorageError> for GitUploadError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err.to_string())
    }
}

/// Domain errors for `gitupload-hooks`.
#[derive(Error, Debug)]
pub enum HookError {
    /// Commit message failed Conventional Commits validation.
    #[error("Conventional commit validation failed: {0}")]
    CommitMessage(String),

    /// Pre-commit scanner flagged a blocked file or secret.
    #[error("Pre-commit guard blocked commit: {0}")]
    PreCommit(String),

    /// Pre-push guard blocked direct push to protected branch.
    #[error("Pre-push guard blocked push: {0}")]
    PrePush(String),
}

impl From<HookError> for GitUploadError {
    fn from(err: HookError) -> Self {
        Self::Hook(err.to_string())
    }
}

/// Standardized Tauri IPC error envelope sent to the Primer UI frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcErrorPayload {
    /// Machine-readable error code (e.g., `"GIT_AUTH_FAILED"`, `"HOOK_REJECTED"`).
    pub code: String,
    /// User-facing descriptive error message.
    pub message: String,
    /// Whether the UI should present a retry action.
    pub recoverable: bool,
}

impl From<GitUploadError> for IpcErrorPayload {
    fn from(err: GitUploadError) -> Self {
        let (code, recoverable) = match &err {
            GitUploadError::Io(_) => ("IO_ERROR", true),
            GitUploadError::Serialization(_) => ("SERIALIZATION_ERROR", false),
            GitUploadError::Config(_) => ("CONFIG_INVALID", true),
            GitUploadError::Vault(_) => ("VAULT_ERROR", true),
            GitUploadError::Git(_) => ("GIT_ENGINE_ERROR", true),
            GitUploadError::DivergenceDetected { .. } => ("GIT_DIVERGENCE_DETECTED", true),
            GitUploadError::GitHubApi { status: 401, .. } => ("GITHUB_AUTH_FAILED", true),
            GitUploadError::GitHubApi { status: 403, .. } => ("ORGANIZATION_FORBIDDEN", false),
            GitUploadError::GitHubApi { status: 404, .. } => ("GITHUB_NOT_FOUND", false),
            GitUploadError::GitHubApi { .. } => ("GITHUB_API_ERROR", true),
            GitUploadError::Hook(_) => ("HOOK_VALIDATION_FAILED", true),
            GitUploadError::GhCli(_) => ("GHCLI_ERROR", true),
            GitUploadError::CloudAi(_) => ("CLOUD_AI_ERROR", true),
            GitUploadError::Storage(_) => ("STORAGE_ERROR", false),
            GitUploadError::SafetyGuard(_) => ("SAFETY_GUARD_BLOCKED", true),
            GitUploadError::Cancelled | GitUploadError::OperationCancelled(_) => {
                ("OPERATION_CANCELLED", true)
            }
            GitUploadError::Internal(_) => ("INTERNAL_ERROR", false),
        };

        Self {
            code: code.to_string(),
            message: err.to_string(),
            recoverable,
        }
    }
}
