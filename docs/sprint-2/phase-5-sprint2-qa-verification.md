# Sprint 2 — Phase 5: QA, Multi-Remote Verification & v2.3 Release Hardening Specification
**Document Identifier:** SPRINT2-PHASE5-QA-VERIFICATION  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Automated Multi-Remote Mock Integration Harnesses, 3-Way Merge Conflict Stress Test Suite & Fuzzing, In-Memory SSH Agent Protocol & Named Pipe Harness, Team Analytics & Heatmap Query Benchmarks, Clippy Zero-Warning & `#![forbid(unsafe_code)]` Compliance Audit, Regression Verification against Sprint 1 Features  
**Target Crates:** All 10 Workspace Crates (`gitupload-*`), Integration Test Suites (`tests/`), Benchmarks (`benches/`), and Tauri Packaging (`src-tauri/`)  
**Timeline:** Days 21–25 (Sprint 2 Final Hardening, Multi-Cloud Quality Assurance & v2.3 Release Verification)  

---

## 1. Phase Overview & Objectives

Phase 5 represents the comprehensive testing, performance validation, and security hardening phase of **Sprint 2**. While Sprint 1 validated the foundational Git engine, local storage, security vault, and basic Tauri IPC, Sprint 2 introduces complex distributed features: multi-remote parallel synchronization across heterogeneous hosting providers (GitHub, GitLab, Bitbucket, Gitea), an interactive visual 3-way merge conflict resolver, an in-process native SSH agent with Windows Named Pipe IPC, and a relational 365-day team contribution analytics subsystem.

The objective of Phase 5 is to rigorously guarantee that these advanced distributed and concurrent subsystems meet all functional requirements, security invariants, and performance SLAs defined in the Software Requirements Specification ([03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md)) and Sprint 2 Master Plan ([README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/README.md)).

### Core Objectives:
1. **Multi-Remote Ephemeral Integration Harness:** Create deterministic, isolated multi-remote test topologies using local bare repositories to test concurrent push/fetch, isolated remote timeouts, and graceful partial failure recovery.
2. **3-Way Merge Conflict Fuzzing & Stress Suite:** Subject the 3-way merge calculation and resolution engine to automated stress tests and randomized conflict fuzzing (up to 5,000 conflicting lines, Unicode/CJK/Vietnamese text, and whitespace variations) ensuring sub-30ms processing and zero memory leaks.
3. **In-Memory SSH Agent Protocol & Pipe Harness:** Validate the pure-Rust SSH agent on Windows Named Pipes against authentic OpenSSH protocol frames (`SSH2_AGENTC_REQUEST_IDENTITIES`, `SSH2_AGENTC_SIGN_REQUEST`), verifying that private keys never leak to disk or log files and zeroize on drop.
4. **Relational Analytics & Heatmap Performance Benchmarks:** Establish Criterion benchmarks guaranteeing that 365-day heatmap queries execute in **< 5.0 milliseconds** and full repository commit history scans over 10,000 commits complete in **< 100 milliseconds**.
5. **Static Security & Code Hygiene Audit:** Verify 100% adherence to `#![forbid(unsafe_code)]` across all domain crates, zero warnings across `cargo clippy --workspace --all-targets -- -D warnings`, zero secret leaks in log outputs or database tables, and zero regressions against Sprint 1 features.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                 Phase 5 Verification Topology                                     |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |        TASK-S2P5-01         |  |         TASK-S2P5-02          |  |       TASK-S2P5-03      |  |
|  | Multi-Remote Sync Harness   |  | 3-Way Merge Conflict Fuzzing  |  | In-Memory SSH Agent     |  |
|  | (Mock Cluster & Failover)   |  | & Resolution Stress Tests     |  | Named Pipe Harness      |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |         TASK-S2P5-04         |                               |
|                                    | Team Analytics Benchmarks    |                               |
|                                    | (< 5ms 365-Day Query SLA)    |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                                                              |          |
|  +------v----------------------+                                        +--------------v-------+  |
|  |        TASK-S2P5-05         |                                        |       TASK-S2P5-06   |  |
|  | Static Security & Clippy   |                                        | Sprint 1 Regression  |  |
|  | Zero-Warning Audit          |                                        | & v2.3 Release DoD   |  |
|  +-----------------------------+                                        +----------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-S2P5-01: Multi-Remote Integration Test Harness & Failover Matrix

- **Task Identifier:** `TASK-S2P5-01`
- **Component:** `tests/multi_remote_sync_test.rs`, `tests/common/mock_remote_cluster.rs`
- **SRS Traceability:** `FR-GIT-017`, `NFR-REL-002`, `NFR-SEC-001`
- **Target Crates:** `gitupload-core`, `gitupload-git`, `gitupload-security`

#### Technical Specification & Implementation Guide
Create an ephemeral test fixture that provisions 4 distinct bare Git repositories acting as mock remotes (`github`, `gitlab`, `bitbucket`, `gitea`). The test harness simulates diverse network conditions, including instant push success, authentication failures, simulated network timeouts, and non-fast-forward rejections.

```rust
// tests/common/mock_remote_cluster.rs
use git2::{Repository, Signature, IndexAddOption};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use gitupload_core::model::remote::{RemoteConfig, RemoteProvider};

pub struct MockRemoteCluster {
    _root_dir: TempDir,
    pub local_repo_path: PathBuf,
    pub github_remote_path: PathBuf,
    pub gitlab_remote_path: PathBuf,
    pub bitbucket_remote_path: PathBuf,
    pub gitea_remote_path: PathBuf,
}

impl MockRemoteCluster {
    pub fn new() -> Self {
        let root = TempDir::new().expect("Failed to create temporary directory for test cluster");
        let root_path = root.path();

        let gh_path = root_path.join("remote_github.git");
        let gl_path = root_path.join("remote_gitlab.git");
        let bb_path = root_path.join("remote_bitbucket.git");
        let gt_path = root_path.join("remote_gitea.git");
        let local_path = root_path.join("local_workdir");

        // Initialize bare repositories for remotes
        Repository::init_bare(&gh_path).expect("Failed to init bare github");
        Repository::init_bare(&gl_path).expect("Failed to init bare gitlab");
        Repository::init_bare(&bb_path).expect("Failed to init bare bitbucket");
        Repository::init_bare(&gt_path).expect("Failed to init bare gitea");

        // Initialize local working repository
        let local_repo = Repository::init(&local_path).expect("Failed to init local repo");

        // Initial commit on main
        let sig = Signature::now("GitUpload QA", "qa@gitupload.desktop").unwrap();
        let file_path = local_path.join("README.md");
        std::fs::write(&file_path, "# Multi-Remote Test Repository\nInitial commit").unwrap();

        let mut index = local_repo.index().unwrap();
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = local_repo.find_tree(tree_id).unwrap();

        local_repo.commit(Some("HEAD"), &sig, &sig, "chore: initial commit", &tree, &[]).unwrap();

        // Add remotes pointing to bare paths
        local_repo.remote("github", gh_path.to_str().unwrap()).unwrap();
        local_repo.remote("gitlab", gl_path.to_str().unwrap()).unwrap();
        local_repo.remote("bitbucket", bb_path.to_str().unwrap()).unwrap();
        local_repo.remote("gitea", gt_path.to_str().unwrap()).unwrap();

        Self {
            _root_dir: root,
            local_repo_path: local_path,
            github_remote_path: gh_path,
            gitlab_remote_path: gl_path,
            bitbucket_remote_path: bb_path,
            gitea_remote_path: gt_path,
        }
    }

    pub fn get_configs(&self) -> Vec<RemoteConfig> {
        vec![
            RemoteConfig {
                name: "github".to_string(),
                url: self.github_remote_path.to_str().unwrap().to_string(),
                provider: RemoteProvider::GitHub,
                sync_enabled: true,
                auto_mirror: true,
                timeout_seconds: 10,
                credential_account: Some("test_gh_user".to_string()),
            },
            RemoteConfig {
                name: "gitlab".to_string(),
                url: self.gitlab_remote_path.to_str().unwrap().to_string(),
                provider: RemoteProvider::GitLab,
                sync_enabled: true,
                auto_mirror: true,
                timeout_seconds: 10,
                credential_account: Some("test_gl_user".to_string()),
            },
            RemoteConfig {
                name: "bitbucket".to_string(),
                url: self.bitbucket_remote_path.to_str().unwrap().to_string(),
                provider: RemoteProvider::Bitbucket,
                sync_enabled: true,
                auto_mirror: true,
                timeout_seconds: 10,
                credential_account: Some("test_bb_user".to_string()),
            },
            RemoteConfig {
                name: "gitea".to_string(),
                url: self.gitea_remote_path.to_str().unwrap().to_string(),
                provider: RemoteProvider::Gitea,
                sync_enabled: true,
                auto_mirror: true,
                timeout_seconds: 10,
                credential_account: Some("test_gt_user".to_string()),
            },
        ]
    }
}
```

```rust
// tests/multi_remote_sync_test.rs
use mock_remote_cluster::MockRemoteCluster;
use gitupload_git::remote::MultiRemoteEngine;
use gitupload_core::model::remote::SyncStatus;
use tokio_util::sync::CancellationToken;

#[tokio::test]
async fn test_parallel_push_all_remotes_succeed() {
    let cluster = MockRemoteCluster::new();
    let configs = cluster.get_configs();
    let cancel = CancellationToken::new();

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel(32);

    let handle = tokio::spawn(async move {
        let mut count = 0;
        while let Some(_event) = progress_rx.recv().await {
            count += 1;
        }
        count
    });

    let results = MultiRemoteEngine::push_all(
        &cluster.local_repo_path,
        "main",
        configs,
        progress_tx,
        cancel,
    ).await.expect("Parallel push failed");

    drop(handle);

    assert_eq!(results.len(), 4);
    for res in &results {
        assert_eq!(res.status, SyncStatus::Success, "Remote {} failed: {:?}", res.remote_name, res.error);
        assert!(res.bytes_transferred > 0);
        assert!(res.duration_ms < 5000);
    }
}

#[tokio::test]
async fn test_isolated_remote_failure_does_not_abort_others() {
    let cluster = MockRemoteCluster::new();
    let mut configs = cluster.get_configs();

    // Corrupt the bitbucket bare remote path so it fails immediately
    configs[2].url = "C:\\invalid\\nonexistent\\path.git".to_string();

    let cancel = CancellationToken::new();
    let (progress_tx, _) = tokio::sync::mpsc::channel(32);

    let results = MultiRemoteEngine::push_all(
        &cluster.local_repo_path,
        "main",
        configs,
        progress_tx,
        cancel,
    ).await.expect("Engine execution failed");

    assert_eq!(results.len(), 4);

    let bb_res = results.iter().find(|r| r.remote_name == "bitbucket").unwrap();
    assert_eq!(bb_res.status, SyncStatus::Failed);
    assert!(bb_res.error.is_some());

    let gh_res = results.iter().find(|r| r.remote_name == "github").unwrap();
    assert_eq!(gh_res.status, SyncStatus::Success);

    let gl_res = results.iter().find(|r| r.remote_name == "gitlab").unwrap();
    assert_eq!(gl_res.status, SyncStatus::Success);

    let gt_res = results.iter().find(|r| r.remote_name == "gitea").unwrap();
    assert_eq!(gt_res.status, SyncStatus::Success);
}
```

---

### Task TASK-S2P5-02: Automated 3-Way Merge Conflict Fuzzing & Stress Test Suite

- **Task Identifier:** `TASK-S2P5-02`
- **Component:** `tests/merge_conflict_stress_test.rs`, `tests/common/conflict_generator.rs`
- **SRS Traceability:** `FR-GIT-018`, `NFR-PERF-001`, `NFR-REL-001`
- **Target Crates:** `gitupload-core`, `gitupload-git`

#### Technical Specification & Implementation Guide
Construct an automated conflict generator that creates synthetically conflicting branches from a shared ancestor commit. Test cases include:
1. **Adjacent and Consecutive Conflicts:** Multiple conflict blocks separated by only 1–2 shared lines.
2. **Whitespace and Line-Ending Variations:** CRLF vs LF divergence with trailing whitespace differences.
3. **Multilingual Unicode & CJK/Vietnamese:** Special character preservation (`Tiếng Việt`, `日本語`, emojis `🚀🔥`) during resolution.
4. **Resolution Strategy Verification:** Programmatically applying resolution choices (`Ours`, `Theirs`, `Both`, `Custom`), executing commit creation, and verifying that the resulting merge commit has exactly two parents.
5. **Stress Fuzzing:** Generating 1,000 files with random conflicting lines to verify that `resolve_3way_conflict` processes under 30ms without panics or memory spikes.

```rust
// tests/merge_conflict_stress_test.rs
use git2::{Repository, Signature, IndexAddOption, Oid};
use tempfile::TempDir;
use gitupload_git::merge::{ThreeWayMergeEngine, ConflictResolutionStrategy, FileResolutionRequest};
use gitupload_core::model::conflict::ResolutionChoice;

struct DivergentRepoFixture {
    _dir: TempDir,
    pub repo: Repository,
    pub base_oid: Oid,
    pub ours_oid: Oid,
    pub theirs_oid: Oid,
}

impl DivergentRepoFixture {
    fn new_with_content(base: &str, ours: &str, theirs: &str) -> Self {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = Signature::now("QA Engineer", "qa@gitupload.desktop").unwrap();

        let file_path = dir.path().join("source.rs");

        // 1. Base commit on main
        std::fs::write(&file_path, base).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("source.rs")).unwrap();
        index.write().unwrap();
        let base_tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let base_commit = repo.commit(Some("HEAD"), &sig, &sig, "feat: base state", &base_tree, &[]).unwrap();

        // 2. Branch 'feature-theirs'
        let base_commit_obj = repo.find_commit(base_commit).unwrap();
        let theirs_branch = repo.branch("feature-theirs", &base_commit_obj, false).unwrap();
        let theirs_ref = theirs_branch.into_reference();

        // 3. Commit 'ours' on main
        std::fs::write(&file_path, ours).unwrap();
        index.add_path(std::path::Path::new("source.rs")).unwrap();
        index.write().unwrap();
        let ours_tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let ours_commit = repo.commit(Some("HEAD"), &sig, &sig, "feat: our modifications", &ours_tree, &[&base_commit_obj]).unwrap();

        // 4. Commit 'theirs' on branch
        repo.set_head(theirs_ref.name().unwrap()).unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
        std::fs::write(&file_path, theirs).unwrap();
        let mut theirs_index = repo.index().unwrap();
        theirs_index.add_path(std::path::Path::new("source.rs")).unwrap();
        theirs_index.write().unwrap();
        let theirs_tree = repo.find_tree(theirs_index.write_tree().unwrap()).unwrap();
        let theirs_commit = repo.commit(Some("HEAD"), &sig, &sig, "feat: their modifications", &theirs_tree, &[&base_commit_obj]).unwrap();

        // Reset HEAD to main (ours)
        repo.set_head("refs/heads/main").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        Self {
            _dir: dir,
            repo,
            base_oid: base_commit,
            ours_oid: ours_commit,
            theirs_oid: theirs_commit,
        }
    }
}

#[test]
fn test_unicode_vietnamese_cjk_conflict_resolution() {
    let base = "fn get_greeting() -> &'static str {\n    \"Hello\"\n}\n";
    let ours = "fn get_greeting() -> &'static str {\n    \"Xin chào thế giới! 🇻🇳\"\n}\n";
    let theirs = "fn get_greeting() -> &'static str {\n    \"こんにちは世界！ 🇯🇵\"\n}\n";

    let fixture = DivergentRepoFixture::new_with_content(base, ours, theirs);

    let conflicts = ThreeWayMergeEngine::detect_conflicts(
        &fixture.repo,
        fixture.ours_oid,
        fixture.theirs_oid,
    ).expect("Conflict detection failed");

    assert_eq!(conflicts.len(), 1);
    let conflict = &conflicts[0];
    assert_eq!(conflict.file_path, "source.rs");
    assert_eq!(conflict.chunks.len(), 1);

    // Resolve choosing 'Ours'
    let resolution = FileResolutionRequest {
        file_path: "source.rs".to_string(),
        chunk_resolutions: vec![
            (0, ResolutionChoice::Ours),
        ],
        custom_content: None,
    };

    let resolved_text = ThreeWayMergeEngine::apply_resolutions(&conflict, &resolution)
        .expect("Resolution failed");

    assert!(resolved_text.contains("Xin chào thế giới! 🇻🇳"));
    assert!(!resolved_text.contains("こんにちは世界"));

    // Finalize merge commit
    let merge_commit_oid = ThreeWayMergeEngine::commit_merge_resolution(
        &fixture.repo,
        fixture.ours_oid,
        fixture.theirs_oid,
        vec![("source.rs".to_string(), resolved_text)],
        "Merge branch 'feature-theirs' with Vietnamese greeting",
    ).expect("Commit creation failed");

    let merge_commit = fixture.repo.find_commit(merge_commit_oid).unwrap();
    assert_eq!(merge_commit.parent_count(), 2);
    assert_eq!(merge_commit.parent_id(0).unwrap(), fixture.ours_oid);
    assert_eq!(merge_commit.parent_id(1).unwrap(), fixture.theirs_oid);
}
```

---

### Task TASK-S2P5-03: In-Memory SSH Agent Protocol & Named Pipe Harness

- **Task Identifier:** `TASK-S2P5-03`
- **Component:** `tests/ssh_agent_protocol_test.rs`, `tests/common/mock_ssh_client.rs`
- **SRS Traceability:** `FR-SEC-003`, `NFR-SEC-001`, `NFR-SEC-002`
- **Target Crates:** `gitupload-security`, `gitupload-git`

#### Technical Specification & Implementation Guide
Verify the Windows Named Pipe SSH agent implementation without requiring an external OpenSSH installation. A pure-Rust mock client connects to the named pipe `\\.\pipe\openssh-ssh-agent`, transmits binary SSH Agent wire-protocol packets, and parses responses.

Key protocol validations:
1. `SSH2_AGENTC_REQUEST_IDENTITIES (11)`: Verifies that the agent returns `SSH2_AGENT_IDENTITIES_ANSWER (12)` containing the public keys for all loaded credentials.
2. `SSH2_AGENTC_SIGN_REQUEST (13)`: Verifies that the agent computes a valid cryptographic signature over the challenge payload using Ed25519 or RSA-SHA256, returning `SSH2_AGENT_SIGN_RESPONSE (14)`.
3. **Security Invariant Verification:**
   - Private key bytes never appear in plaintext in filesystem paths.
   - Keys are kept in `secrecy::SecretBox` and wiped with `zeroize::Zeroize` on drop.
   - Named pipe security attributes prevent access from non-elevated cross-session users.

```rust
// tests/ssh_agent_protocol_test.rs
use gitupload_security::ssh::{NativeSshAgent, SshKeyType, GeneratedKeypair};
use gitupload_security::ssh::protocol::{SSH2_AGENTC_REQUEST_IDENTITIES, SSH2_AGENT_IDENTITIES_ANSWER};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ClientOptions;
use std::time::Duration;

#[tokio::test]
async fn test_ssh_agent_pipe_handshake_and_identity_query() {
    let test_pipe_name = r"\\.\pipe\gitupload-test-agent-pipe";

    // 1. Generate an ephemeral Ed25519 keypair
    let keypair = GeneratedKeypair::generate(SshKeyType::Ed25519, "qa-agent-test@gitupload.desktop")
        .expect("Key generation failed");

    // 2. Start agent bound to test pipe
    let agent = NativeSshAgent::start_on_pipe(test_pipe_name)
        .expect("Failed to start test named pipe agent");

    agent.add_identity(keypair.clone()).expect("Failed to add key to agent");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // 3. Connect mock client to named pipe
    let mut client = ClientOptions::new()
        .open(test_pipe_name)
        .expect("Mock client failed to open named pipe");

    // Construct SSH2_AGENTC_REQUEST_IDENTITIES message (4-byte length + 1-byte opcode = 5 bytes total)
    let mut request_buf = Vec::new();
    request_buf.extend_from_slice(&1u32.to_be_bytes()); // Length = 1 byte
    request_buf.push(SSH2_AGENTC_REQUEST_IDENTITIES);

    client.write_all(&request_buf).await.expect("Failed to write request");

    // Read response length
    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.expect("Failed to read resp len");
    let resp_len = u32::from_be_bytes(len_buf) as usize;

    let mut body_buf = vec![0u8; resp_len];
    client.read_exact(&mut body_buf).await.expect("Failed to read resp body");

    assert_eq!(body_buf[0], SSH2_AGENT_IDENTITIES_ANSWER);

    // Read key count
    let key_count = u32::from_be_bytes([body_buf[1], body_buf[2], body_buf[3], body_buf[4]]);
    assert_eq!(key_count, 1);

    // Stop agent and verify pipe teardown
    agent.shutdown().expect("Agent shutdown failed");
}
```

---

### Task TASK-S2P5-04: Team Analytics & 365-Day Heatmap Query Benchmarks

- **Task Identifier:** `TASK-S2P5-04`
- **Component:** `benches/analytics_benchmark.rs`, `tests/analytics_query_test.rs`
- **SRS Traceability:** `FR-SET-001`, `NFR-PERF-003`, `NFR-PERF-004`
- **Target Crates:** `gitupload-storage`, `gitupload-git`

#### Technical Specification & Implementation Guide
Establish performance benchmarks validating that the relational SQLite analytics schema and Git revwalk metric collectors meet their SLAs under enterprise-scale repository loads:
1. **Heatmap Retrieval SLA:** Retrieving 365 daily aggregated commit buckets must execute in **< 5.0 milliseconds**.
2. **Git History Scan SLA:** Parsing commit metadata, author email, timestamp, and line churn (+/- additions/deletions) across 10,000 commits must complete in **< 100 milliseconds**.
3. **Classroom Leaderboard SLA:** Evaluating student streaks, total commits, and homework timeliness for 200 students must complete in **< 15.0 milliseconds**.

```rust
// benches/analytics_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gitupload_storage::database::DatabaseManager;
use gitupload_storage::repository::AnalyticsRepository;
use gitupload_core::model::analytics::{DailyCommitStat, ClassroomStudentSubmission};
use chrono::{Utc, Duration};

fn setup_benchmark_db(commit_count: usize) -> (DatabaseManager, TempDir) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("bench_analytics.db");
    let mgr = DatabaseManager::new(&db_path).unwrap();

    let mut conn = mgr.get_connection().unwrap();
    let tx = conn.transaction().unwrap();

    let now = Utc::now();
    for i in 0..commit_count {
        let date = (now - Duration::days((i % 365) as i64)).format("%Y-%m-%d").to_string();
        tx.execute(
            "INSERT INTO daily_commit_stats (repo_id, commit_date, commit_count, additions, deletions, author_email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "repo-bench-1",
                date,
                (i % 12) + 1,
                (i % 50) * 10,
                (i % 20) * 5,
                format!("student_{}@university.edu", i % 30),
            ],
        ).unwrap();
    }
    tx.commit().unwrap();

    (mgr, dir)
}

fn benchmark_365_day_heatmap_query(c: &mut Criterion) {
    let (mgr, _dir) = setup_benchmark_db(50_000);
    let repo = AnalyticsRepository::new(mgr);

    c.bench_function("query_365_day_heatmap_50k_rows", |b| {
        b.iter(|| {
            let heatmap = repo.get_365_day_heatmap(black_box("repo-bench-1"))
                .expect("Heatmap query failed");
            assert_eq!(heatmap.len(), 365);
        })
    });
}

fn benchmark_classroom_leaderboard_query(c: &mut Criterion) {
    let (mgr, _dir) = setup_benchmark_db(50_000);
    let repo = AnalyticsRepository::new(mgr);

    c.bench_function("query_classroom_leaderboard_rankings", |b| {
        b.iter(|| {
            let rankings = repo.get_classroom_leaderboard(black_box("repo-bench-1"), 50)
                .expect("Leaderboard query failed");
            assert!(!rankings.is_empty());
        })
    });
}

criterion_group!(benches, benchmark_365_day_heatmap_query, benchmark_classroom_leaderboard_query);
criterion_main!(benches);
```

---

### Task TASK-S2P5-05: Static Security, Clippy Zero-Warning & `#![forbid(unsafe_code)]` Audit

- **Task Identifier:** `TASK-S2P5-05`
- **Component:** `.github/workflows/ci.yml`, `scripts/audit_crates.ps1`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-MNT-001`, `NFR-MNT-002`
- **Target Crates:** All 10 Workspace Crates

#### Technical Specification & Implementation Guide
Implement an automated static verification checklist and verification script ensuring absolute compliance with GitUpload engineering standards:
1. **Zero Clippy Warnings:** `cargo clippy --workspace --all-targets -- -D warnings` must complete with exit code 0.
2. **`#![forbid(unsafe_code)]` Enforcement:** Every crate root (`crates/*/src/lib.rs` and `crates/gitupload-app/src/main.rs`) must explicitly declare `#![forbid(unsafe_code)]`.
   - *Sole Bounded Exception:* Windows Named Pipe and Mutex initialization in `gitupload-app/src/win32.rs` and `gitupload-security/src/ssh/named_pipe.rs` with explicit auditing comments.
3. **Zero Plaintext Credentials in Logs or Storage:**
   - Scan SQLite schema migrations: verify that no token, secret, or password column exists in plaintext.
   - Scan tracing log filters: verify that `sanitize_log_message` scrubs all GitHub PATs (`ghp_*`, `github_pat_*`) and Git remote URLs (`https://***@github.com`).
4. **Memory Hygiene Verification:**
   - Verify that all SSH private key types and bearer tokens implement `zeroize::Zeroize` and `secrecy::SecretString`.

```powershell
# scripts/audit_crates.ps1
# Automated audit script for Sprint 2 CI validation
$ErrorActionPreference = "Stop"

Write-Host "=== 1. Checking Format and Clippy Warnings ===" -ForegroundColor Cyan
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

Write-Host "=== 2. Enforcing #![forbid(unsafe_code)] ===" -ForegroundColor Cyan
$crateRoots = Get-ChildItem -Path "crates" -Recurse -Include "lib.rs", "main.rs"
foreach ($file in $crateRoots) {
    $content = Get-Content $file.FullName -Raw
    if ($content -notmatch "#\!\[forbid\(unsafe_code\)\]") {
        # Check if it's the permitted win32 exception
        if ($file.FullName -match "gitupload-app" -or $file.FullName -match "gitupload-security") {
            Write-Host "  [ALLOWED FFI EXCEPTION] $($file.FullName)" -ForegroundColor Yellow
        } else {
            Write-Error "Crate root missing `#![forbid(unsafe_code)]`: $($file.FullName)"
        }
    } else {
        Write-Host "  [OK] $($file.FullName)" -ForegroundColor Green
    }
}

Write-Host "=== 3. Scanning for Hardcoded Tokens or Keys ===" -ForegroundColor Cyan
$suspiciousPatterns = @("ghp_", "github_pat_", "-----BEGIN OPENSSH PRIVATE KEY-----")
foreach ($pat in $suspiciousPatterns) {
    $matches = Select-String -Path "crates\*\src\**\*.rs", "ui\src\**\*.ts" -Pattern $pat -Exclude "*.md"
    if ($matches) {
        Write-Error "Found potential hardcoded secret matching pattern '$pat':`n$matches"
    }
}

Write-Host "=== Phase 5 Static Audit Successfully Passed ===" -ForegroundColor Green
```

---

### Task TASK-S2P5-06: Regression Verification against Sprint 1 & Release v2.3 Hardening

- **Task Identifier:** `TASK-S2P5-06`
- **Component:** `tests/sprint1_regression_test.rs`, `src-tauri/tauri.conf.json`
- **SRS Traceability:** `FR-GIT-001` through `FR-GIT-016`, `NFR-PERF-001`, `NFR-REL-001`
- **Target Crates:** All 10 Workspace Crates, `src-tauri/`, `ui/`

#### Technical Specification & Implementation Guide
Ensure that none of the Sprint 2 enhancements degrade the core functionality established in Sprint 1:
1. **Core Git Operations Regression Suite:**
   - Verify single-remote commit, branch, tag, cherry-pick, and stash workflows operate identically.
   - Verify safe sync with uncommitted local work (auto-stash before pull, re-apply after pull) remains intact.
2. **AI Commit Engine Compatibility:**
   - Confirm that the Cloud AI commit generator (`gitupload-ai`) continues generating Conventional Commits and adheres to commit-msg hook validations.
3. **Tauri IPC Command Registry Verification:**
   - Validate that all IPC commands from Sprint 1 and Sprint 2 are registered in `gitupload-app/src/lib.rs` without name collisions or missing deserializers.
4. **Desktop Packaging & Footprint Verification:**
   - Windows installer build (`cargo tauri build`) produces an NSIS installer under **15 MB**.
   - Application cold startup launches to interactive Primer UI in **< 100 ms**.
   - Idle memory footprint remains strictly under **28 MB Private Working Set RAM** with `mimalloc`.

---

## 3. Sprint 2 Acceptance Criteria (AC-S2P5-01 to AC-S2P5-08)

### AC-S2P5-01: Multi-Remote Concurrent Push Completeness
- **Given** a local repository with 4 configured remotes (GitHub, GitLab, Bitbucket, Gitea),
- **When** the user executes "Push to All Remotes",
- **Then** all 4 remotes receive commits concurrently via Tokio `JoinSet`, progress events stream to the UI at 60Hz, and total push duration finishes within **1.2x of the slowest remote**.

### AC-S2P5-02: Isolated Multi-Remote Network Failure
- **Given** 4 configured remotes where 1 remote (GitLab) is unreachable or times out,
- **When** a multi-remote push or fetch is triggered,
- **Then** the unreachable remote reports `SyncStatus::Failed` with an isolated error message, while the remaining 3 remotes complete successfully with `SyncStatus::Success`, and the UI displays an aggregate summary badge `3/4 Synced`.

### AC-S2P5-03: Visual 3-Way Merge Conflict Detection
- **Given** a branch merge where file `app.rs` has conflicting modifications from both branches,
- **When** the 3-way merge engine detects the conflict,
- **Then** `detect_conflicts` parses base, current, and incoming trees in **< 30 milliseconds**, emitting structured conflict chunks with exact line numbers and diff boundaries.

### AC-S2P5-04: 1-Click Chunk Resolution & Two-Parent Commit
- **Given** a conflicted file in the Monaco 3-pane merge editor,
- **When** the user clicks "Accept Current" or "Accept Incoming" for all chunks and clicks "Finalize Merge",
- **Then** the engine stages the resolved index, writes the tree, creates a merge commit with exactly two parent OIDs, and clears the conflict state without leaving dirty index artifacts.

### AC-S2P5-05: In-Memory SSH Agent OpenSSH Protocol Compliance
- **Given** an Ed25519 or RSA-4096 key generated within GitUpload Desktop,
- **When** an SSH client sends an `SSH2_AGENTC_REQUEST_IDENTITIES` or `SSH2_AGENTC_SIGN_REQUEST` over the Windows Named Pipe `\\.\pipe\openssh-ssh-agent`,
- **Then** the native agent returns valid protocol responses, signs Git commit or push payloads in memory, and leaves zero private key bytes on disk or in system swap.

### AC-S2P5-06: 365-Day Contribution Heatmap Query Latency
- **Given** a repository with 50,000 historical commits spanning the past 365 days,
- **When** the user opens the Team Analytics view,
- **Then** the SQLite query aggregating daily commit counts and line churn returns complete 365-day bucket data in **< 5.0 milliseconds**, and the Primer UI renders the 5-tier green heatmap at 60 FPS.

### AC-S2P5-07: Classroom Leaderboard & Homework Streak Calculation
- **Given** a classroom repository syllabus with weekly exercise deadlines,
- **When** student submission analytics are evaluated,
- **Then** the engine calculates active streak days, on-time submission percentages, and leaderboard rank accurately matching student commit timestamps normalized to UTC.

### AC-S2P5-08: Zero-Warning and Memory Invariant Enforcement
- **Given** the entire Sprint 2 codebase across all 10 workspace crates,
- **When** `cargo clippy --workspace --all-targets -- -D warnings` and runtime memory audits are executed,
- **Then** zero warnings are emitted, all crates enforce `#![forbid(unsafe_code)]` (excluding documented win32 pipe/mutex FFI), and idle application RAM remains **< 28 MB**.

---

## 4. Definition of Done (DoD) Checklist for Sprint 2

| Verification Item | Target Standard | Verification Method | Status |
| :--- | :--- | :--- | :--- |
| **Workspace Compilation** | Zero compile errors | `cargo check --workspace --all-targets` | [ ] Pending Final Run |
| **Clippy Linter Audit** | Zero warnings permitted | `cargo clippy --workspace --all-targets -- -D warnings` | [ ] Pending Final Run |
| **Unit Test Coverage** | > 85% branch coverage | `cargo test --workspace` | [ ] Pending Final Run |
| **Multi-Remote Tests** | 4-remote parallel test passes | `cargo test -p gitupload-git --test multi_remote_sync_test` | [ ] Pending Final Run |
| **3-Way Merge Fuzzing** | 1,000 randomized files pass | `cargo test -p gitupload-git --test merge_conflict_stress_test` | [ ] Pending Final Run |
| **SSH Agent Named Pipe** | Wire protocol round-trip | `cargo test -p gitupload-security --test ssh_agent_protocol_test` | [ ] Pending Final Run |
| **Heatmap Performance** | < 5.0 ms query latency | `cargo bench --bench analytics_benchmark` | [ ] Pending Final Run |
| **Memory Invariant** | < 28 MB Working Set RAM | VMMap / Process Explorer inspection | [ ] Pending Final Run |
| **Cold Startup SLA** | < 100 ms to interactive | Visual Studio Diagnostics / Performance Profiler | [ ] Pending Final Run |
| **Static Security Audit** | Zero token/key leaks in logs | Automated regex audit across codebase & traces | [ ] Pending Final Run |

---

> [!NOTE]
> Upon satisfying all acceptance criteria (`AC-S2P5-01` through `AC-S2P5-08`) and completing the DoD checklist, Sprint 2 is certified release-ready for desktop packaging as **GitUpload Desktop v2.3.0**.
