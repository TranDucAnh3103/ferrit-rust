//! Multi-remote mirroring, endpoint health, and aggregate progress DTOs.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported Git hosting provider kinds for remote endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteProviderType {
    /// GitHub.com or GitHub Enterprise.
    GitHub,
    /// GitLab.com or self-hosted GitLab.
    GitLab,
    /// Atlassian Bitbucket Cloud/Server.
    Bitbucket,
    /// Gitea / Forgejo self-hosted server.
    Gitea,
    /// Generic Git HTTP/SSH server.
    Custom,
}

/// Synchronization behavior assigned to a remote endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPolicy {
    /// Fetch and push updates symmetrically.
    Bidirectional,
    /// Push outgoing commits only.
    PushOnly,
    /// Mirror all refs as read-safe remote backup.
    BackupMirror,
}

/// Configuration for a single repository remote target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteEndpoint {
    /// Git remote alias (`origin`, `gitlab`, `gitea-backup`).
    pub name: String,
    /// Clean token-free remote URL (`https://...` or `git@...` or local path).
    pub url: String,
    /// Hosting provider classification.
    pub provider: RemoteProviderType,
    /// Synchronization policy.
    pub sync_policy: SyncPolicy,
    /// Credential Vault account alias mapped for this host.
    pub auth_account: String,
    /// Whether this remote participates in parallel mirroring.
    pub is_enabled: bool,
    /// Isolated operation timeout in seconds.
    pub timeout_seconds: u32,
}

/// Completion outcome status of a multi-remote operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    /// Remote received all refs cleanly.
    Success,
    /// Remote failed due to auth, divergence, or network error.
    Failed,
    /// Remote exceeded configured timeout without stalling peers.
    TimedOut,
    /// Remote disabled or skipped by policy.
    Skipped,
}

/// Per-remote outcome record returned by parallel push/fetch coordinators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemotePushResult {
    /// Remote alias name.
    pub remote_name: String,
    /// True when operation succeeded cleanly.
    pub is_success: bool,
    /// Structured status code.
    pub status: SyncStatus,
    /// Total pack objects/bytes processed (reported by transport callbacks).
    pub bytes_transferred: usize,
    /// Sanitized error details when failed.
    pub error_message: Option<String>,
    /// Elapsed milliseconds for this remote task.
    pub duration_ms: u64,
}

/// Diagnostic connectivity health state returned by lightweight remote probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteHealthStatus {
    /// Handshake succeeded within timeout.
    Healthy,
    /// Authentication rejected (401/403 or invalid SSH identity).
    AuthFailed,
    /// Handshake timed out.
    Timeout,
    /// DNS/TCP/repository path unreachable.
    Unreachable,
}

/// Result of a fast pre-flight remote connectivity probe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteHealthReport {
    /// Remote alias name.
    pub remote_name: String,
    /// Health classification.
    pub status: RemoteHealthStatus,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Diagnostic detail (token-sanitized).
    pub message: String,
}

/// Micro-batched 60Hz aggregate progress payload across concurrent remote streams.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiRemoteProgressPayload {
    /// Map of `remote_name -> percentage (0..=100)`.
    pub remotes: BTreeMap<String, u32>,
    /// Aggregate mean completion across all active remotes (`0..=100`).
    pub overall_percentage: u32,
}
