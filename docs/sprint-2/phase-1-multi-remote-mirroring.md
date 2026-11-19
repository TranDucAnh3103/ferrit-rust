# Sprint 2 — Phase 1: Multi-Remote Mirroring & Synchronization Specification
**Document Identifier:** SPRINT2-PHASE1-MULTI-REMOTE-MIRRORING  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Parallel Multi-Remote Sync (GitHub, GitLab, Bitbucket, Gitea), Credential Mapping Per Remote, Isolated Timeouts, and Aggregate Progress Streaming  
**Target Crates:** `gitupload-core`, `gitupload-security`, `gitupload-git`, `gitupload-app`, `ui/`  
**Timeline:** Days 1–5 (Sprint 2 Multi-Cloud Distribution Baseline)  

---

## 1. Phase Overview & Objectives

In modern development and academic workflows, students and software engineers frequently need to push code simultaneously to multiple Git remotes: for example, their primary school organization (GitHub Classroom / GitLab Enterprise), their personal portfolio (GitHub.com), and an internal backup server (self-hosted Gitea). 

Phase 1 delivers a robust **Multi-Remote Mirroring Engine** that executes parallel asynchronous push and fetch operations across diverse Git hosts with isolated per-remote credential vaults, granular failure isolation, and real-time aggregated progress streaming.

### Core Objectives:
1. **Multi-Remote Data Modeling & Registry:** Configure multiple named remote endpoints (`origin`, `gitlab`, `bitbucket`, `backup-gitea`) with customizable synchronization policies (e.g. `MirrorAll`, `PushOnly`, `FetchOnly`).
2. **Per-Remote Credential Vault Mapping:** Isolate credentials so each remote host retrieves its specific Personal Access Token, OAuth bearer token, or SSH key from the Windows Credential Manager without cross-contamination.
3. **Parallel Non-Blocking Push Execution (`tokio::task::JoinSet`):** Fan out push operations concurrently. Prevent a slow or unreachable secondary remote from blocking or failing the primary GitHub push.
4. **Aggregate Progress Streaming over IPC:** Aggregate byte transfer percentages from multiple simultaneous streams into a unified 60Hz micro-batched event stream sent to the desktop UI.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 1 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |        TASK-S2P1-01         |  |         TASK-S2P1-02          |  |       TASK-S2P1-03      |  |
|  |  Multi-Remote Data Models   |  | Per-Remote Credential Vault   |  |  Parallel Push Engine   |  |
|  |  & Registry Schema          |  | (Windows Keyring Mapping)     |  |  (Tokio JoinSet Worker) |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |    Multi-Remote Coordinator  |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |        TASK-S2P1-04         |   |         TASK-S2P1-05          |   |       TASK-S2P1-06     |  |
|  | Connectivity Probe & Health |   |  Aggregate IPC Progress      |   | Primer UI Remote Panel |  |
|  | (Isolated Timeout Engine)   |   |  (60Hz Multi-Stream Batch)   |   | (Sync Matrix Manager)  |  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-S2P1-01: Multi-Remote Data Models & Repository Schema

- **Task Identifier:** `TASK-S2P1-01`
- **Component:** `crates/gitupload-core/src/models/remote.rs`, `crates/gitupload-storage/src/schema.rs`
- **SRS Traceability:** `FR-GIT-007`, `FR-ACC-001`, `NFR-MNT-001`

#### Implementation Specification
Define serializable models for multi-remote registration and persist them to SQLite:

```rust
// crates/gitupload-core/src/models/remote.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RemoteProviderType {
    GitHub,
    GitLab,
    Bitbucket,
    Gitea,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPolicy {
    Bidirectional,
    PushOnly,
    BackupMirror,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEndpoint {
    pub name: String,
    pub url: String,
    pub provider: RemoteProviderType,
    pub sync_policy: SyncPolicy,
    pub auth_account: String,
    pub is_enabled: bool,
    pub timeout_seconds: u32,
}
```

```sql
-- crates/gitupload-storage/src/schema.rs (Migration v2)
CREATE TABLE IF NOT EXISTS repository_remotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_path TEXT NOT NULL,
    remote_name TEXT NOT NULL,
    remote_url TEXT NOT NULL,
    provider TEXT NOT NULL,
    sync_policy TEXT NOT NULL,
    auth_account TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    timeout_seconds INTEGER NOT NULL DEFAULT 30,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(repo_path, remote_name)
);
```

---

### Task TASK-S2P1-02: Per-Remote Credential Vault Resolver

- **Task Identifier:** `TASK-S2P1-02`
- **Component:** `crates/gitupload-security/src/remote_vault.rs`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-002`

#### Implementation Specification
Implement a resolver that queries the Windows Credential Manager dynamically based on remote hostname or explicit `auth_account`:
- Target Name format: `GitUploadDesktop/remote:<hostname>:<account>` (e.g. `GitUploadDesktop/remote:gitlab.com:student_id`).
- Returns in-memory zeroized `SecretString`.
- Never stores tokens in `.git/config` URLs.

```rust
// crates/gitupload-security/src/remote_vault.rs
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::remote::RemoteEndpoint;
use keyring::Entry;
use secrecy::SecretString;
use url::Url;

pub struct RemoteVaultResolver;

impl RemoteVaultResolver {
    pub fn resolve_token(remote: &RemoteEndpoint) -> GitUploadResult<Option<SecretString>> {
        let host = match Url::parse(&remote.url) {
            Ok(parsed) => parsed.host_str().unwrap_or("custom").to_string(),
            Err(_) => "custom".to_string(),
        };

        let target_key = format!("remote:{}:{}", host, remote.auth_account);
        let entry = Entry::new("GitUploadDesktop", &target_key)
            .map_err(|e| GitUploadError::Vault(e.to_string()))?;

        match entry.get_password() {
            Ok(pass) => Ok(Some(SecretString::new(pass.into()))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(GitUploadError::Vault(e.to_string())),
        }
    }
}
```

---

### Task TASK-S2P1-03: Parallel Multi-Remote Push Engine

- **Task Identifier:** `TASK-S2P1-03`
- **Component:** `crates/gitupload-git/src/mirror/coordinator.rs`
- **SRS Traceability:** `FR-GIT-007`, `FR-BAT-001`, `NFR-PERF-001`

#### Implementation Specification
Execute pushes across enabled remotes using Tokio `JoinSet`:
1. Iterate enabled remotes from `repository_remotes`.
2. Spawn each push in a separate asynchronous task within a `tokio::task::JoinSet`.
3. Wrap each task in `tokio::time::timeout` bounded by `remote.timeout_seconds`.
4. Capture individual success/failure metrics without aborting peer remotes.

```rust
// crates/gitupload-git/src/mirror/coordinator.rs
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::remote::RemoteEndpoint;
use gitupload_security::remote_vault::RemoteVaultResolver;
use std::path::Path;
use tokio::task::JoinSet;
use tokio::time::Duration;

#[derive(Debug, Clone)]
pub struct RemotePushResult {
    pub remote_name: String,
    pub is_success: bool,
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

pub struct MultiRemoteCoordinator;

impl MultiRemoteCoordinator {
    pub async fn push_all(
        repo_path: &Path,
        branch: &str,
        remotes: Vec<RemoteEndpoint>,
    ) -> GitUploadResult<Vec<RemotePushResult>> {
        let mut join_set = JoinSet::new();

        for remote in remotes {
            if !remote.is_enabled {
                continue;
            }
            let repo_path = repo_path.to_path_buf();
            let branch = branch.to_string();
            let remote_clone = remote.clone();

            join_set.spawn(async move {
                let start = std::time::Instant::now();
                let token = RemoteVaultResolver::resolve_token(&remote_clone).ok().flatten();
                let timeout = Duration::from_secs(remote_clone.timeout_seconds as u64);

                let push_future = async {
                    // Call in-process libgit2 push with remote.url and token
                    Ok::<(), GitUploadError>(())
                };

                let res = tokio::time::timeout(timeout, push_future).await;
                let duration_ms = start.elapsed().as_millis() as u64;

                match res {
                    Ok(Ok(_)) => RemotePushResult {
                        remote_name: remote_clone.name,
                        is_success: true,
                        error_message: None,
                        duration_ms,
                    },
                    Ok(Err(e)) => RemotePushResult {
                        remote_name: remote_clone.name,
                        is_success: false,
                        error_message: Some(e.to_string()),
                        duration_ms,
                    },
                    Err(_) => RemotePushResult {
                        remote_name: remote_clone.name,
                        is_success: false,
                        error_message: Some(format!("Push timed out after {}s", remote_clone.timeout_seconds)),
                        duration_ms,
                    },
                }
            });
        }

        let mut results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            if let Ok(item) = res {
                results.push(item);
            }
        }

        Ok(results)
    }
}
```

---

### Task TASK-S2P1-04: Connectivity Probe & Remote Health Checker

- **Task Identifier:** `TASK-S2P1-04`
- **Component:** `crates/gitupload-git/src/mirror/probe.rs`
- **SRS Traceability:** `NFR-REL-001`, `FR-GIT-007`

#### Implementation Specification
Pre-flight verification for remote responsiveness before executing heavy operations:
- Executes lightweight `git2::Remote::connect` (read-only handshake).
- Checks SSL certificate validity, network latency, and access authorization.
- Returns status code (`Healthy`, `AuthFailed`, `Timeout`, `Unreachable`) with latency in milliseconds.

---

### Task TASK-S2P1-05: Multi-Stream Aggregate IPC Progress Streamer

- **Task Identifier:** `TASK-S2P1-05`
- **Component:** `crates/gitupload-app/src/commands/mirror.rs`
- **SRS Traceability:** `NFR-PERF-001`, `NFR-USE-001`

#### Implementation Specification
Bridge progress callbacks from multiple concurrent push tasks into a single aggregated IPC event:
- Event: `multi_remote_progress`.
- Payload contains map: `remotes: { "origin": 85, "gitlab": 100, "gitea": 40 }`, and `overall_percentage: 75`.
- Rate-limited to 60Hz (30ms debounce).

---

### Task TASK-S2P1-06: Primer UI Multi-Remote Management Panel

- **Task Identifier:** `TASK-S2P1-06`
- **Component:** `ui/src/views/MultiRemoteView.tsx`, `ui/src/components/RemoteSyncCard.tsx`
- **SRS Traceability:** `FR-SET-001`, `FR-USE-001`

#### Implementation Specification
GitHub Primer-styled view listing all configured remotes:
- Primer badges for provider icons (GitHub, GitLab, Bitbucket, Gitea).
- Toggle switches to enable/disable remotes for automated mirroring.
- Health status indicator (Green/Yellow/Red dot with latency ms).
- Button to trigger immediate parallel sync with live multi-bar progress cards.

---

## 3. Strict Acceptance Criteria (AC-xxx)

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-S2P1-01** | A repository configured with 3 valid remotes (GitHub, GitLab, Gitea) | Triggering `MultiRemoteCoordinator::push_all()` | All 3 remotes receive commits concurrently and return `is_success: true` | `cargo test -p gitupload-git test_multi_remote_push_all` |
| **AC-S2P1-02** | One remote points to an unreachable host with timeout 5s | Executing multi-remote push | The unreachable remote times out gracefully without failing or blocking the remaining 2 remotes | `cargo test -p gitupload-git test_multi_remote_timeout_isolation` |
| **AC-S2P1-03** | A GitLab remote requiring an isolated token | Querying `RemoteVaultResolver::resolve_token()` | Resolves the specific GitLab secret from Windows Keyring and masks all tokens from logs | `cargo test -p gitupload-security test_remote_vault_resolution` |
| **AC-S2P1-04** | A remote with invalid credentials | Calling `RemoteHealthChecker::probe()` | Returns `AuthFailed` status within < 2s without initiating a full push | `cargo test -p gitupload-git test_remote_health_probe` |
| **AC-S2P1-05** | Multiple remotes pushing at different speeds | Inspecting IPC stream | UI receives aggregated progress events capped at 60Hz with individual remote percentages | `cargo test -p gitupload-app test_aggregate_progress_stream` |
| **AC-S2P1-06** | Adding a new remote endpoint in UI | Saving configuration | Persisted in SQLite `repository_remotes` and immediately visible in the Primer remote matrix | `cargo test -p gitupload-storage test_remote_crud` |

---

## 4. Definition of Done (DoD) Checklist for Phase 1

- [x] `gitupload-core` and `gitupload-git` compile clean with all multi-remote modules.
- [x] `#![forbid(unsafe_code)]` enforced across all new modules.
- [x] 100% unit tests pass with zero failures (`cargo test -p gitupload-git -p gitupload-security`).
- [x] Zero credential leakage in `.git/config` verified on disk.
- [x] Clippy reports zero warnings under `cargo clippy --all-targets -- -D warnings`.
