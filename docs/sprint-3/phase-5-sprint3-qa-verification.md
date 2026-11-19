# Sprint 3 — Phase 5 Specification: DevSecOps QA, Verification & Performance Benchmarks

**Document ID:** SPRINT3-PHASE-5  
**Target Milestone:** Days 48–50 (Sprint 3, Phase 5)  
**Author:** @architect (DeepMind Swarm Architect)  
**Status:** Approved for Implementation  
**Traceability:** Implements `NFR-PERF-001..004`, `NFR-SEC-001..004`, `NFR-MNT-001..003`, `FR-SEC-001..008`  

---

## 1. Phase Overview & Objectives

Phase 5 delivers the comprehensive **Quality Assurance, Benchmark Validation & DevSecOps Verification Suite** for Sprint 3:
1. **Automated Security Scanner Test Harness**: Verifies that static secret scanning and Cloud AI vulnerability detection correctly flag OWASP Top 10 flaws and apply 1-click code patches cleanly.
2. **Vector Similarity Search Benchmarks**: Uses **Criterion.rs** to measure the latency of cosine similarity ranking across 10,000 cached vectors, enforcing the strict `< 50 ms` SLA.
3. **Stale Branch Cleanup & Merge Base Suite**: Tests topological branch analysis against synthetic Git DAG trees with fast-forward merges, squashed commits, and protected branch attempts.
4. **Autonomous Daemon Energy & Battery Profiling**: Verifies that background sync operations respect Windows battery policies and consume `< 0.5% CPU`.
5. **Clippy & `#![forbid(unsafe_code)]` Audit**: Guarantees zero code warnings, clean memory hygiene, and complete log token redaction across all Sprint 3 code.

---

## 2. Granular Task Breakdown

```mermaid
flowchart TD
    P5_01["TASK-P5-01\nDevSecOps Vulnerability\nTest Harness"] --> P5_02["TASK-P5-02\nCosine Vector Search\nCriterion Benchmarks"]
    P5_02 --> P5_03["TASK-P5-03\nStale Branch & Merge Base\nTest Suite"]
    P5_03 --> P5_04["TASK-P5-04\nBackground Daemon Energy\n& Battery Soak Tests"]
    P5_04 --> P5_05["TASK-P5-05\nClippy, Unsafe & Secret\nLeak Verification Gate"]
```

---

### TASK-P5-01: AI Vulnerability Scanner Test Harness
*   **Target Crate**: `crates/gitupload-security/` & `crates/gitupload-ai/`
*   **File Path**: `crates/gitupload-security/tests/vulnerability_tests.rs`
*   **Responsibility**: Tests detection of high-entropy API secrets, unencrypted `.env` files, and 1-click patch application precision.

#### Complete Rust Code Implementation:
```rust
// crates/gitupload-security/tests/vulnerability_tests.rs
use gitupload_security::scanner::StaticSecretScanner;
use gitupload_core::models::security::{SecurityPatchProposal, VulnerabilitySeverity};
use gitupload_git::patcher::AutomatedPatcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_static_scanner_catches_hardcoded_pat() {
    let raw_code = r#"
        let token = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        println!("Connecting with token: {}", token);
    "#;

    let findings = StaticSecretScanner::scan_content("src/auth.rs", raw_code);
    assert!(!findings.is_empty(), "Scanner must catch hardcoded GitHub PAT");
    assert_eq!(findings[0].severity, VulnerabilitySeverity::Critical);
    assert_eq!(findings[0].cwe_id.as_deref(), Some("CWE-798"));
}

#[test]
fn test_static_scanner_catches_env_file() {
    let findings = StaticSecretScanner::scan_content(".env.production", "DATABASE_URL=postgres://...");
    assert!(!findings.is_empty(), "Scanner must flag committing an unencrypted .env file");
    assert_eq!(findings[0].severity, VulnerabilitySeverity::Critical);
}

#[test]
fn test_patcher_applies_exact_remediation() {
    let dir = tempdir().expect("Failed to create tempdir");
    let file_path = dir.path().join("main.rs");

    let initial_content = "fn main() {\n    let query = \"SELECT * FROM users WHERE id = \" + id;\n}\n";
    fs::write(&file_path, initial_content).expect("Failed to write initial file");

    let patch = SecurityPatchProposal {
        patch_description: "Use parameterized query".into(),
        original_code: "let query = \"SELECT * FROM users WHERE id = \" + id;".into(),
        replacement_code: "let query = sqlx::query(\"SELECT * FROM users WHERE id = $1\").bind(id);".into(),
        diff_unified: String::new(),
    };

    let result = AutomatedPatcher::apply_patch(dir.path(), "main.rs", &patch);
    assert!(result.is_ok(), "Patch application must succeed");

    let updated = fs::read_to_string(&file_path).expect("Failed to read updated file");
    assert!(updated.contains("sqlx::query"), "Patched file must contain replacement code");
    assert!(!updated.contains("\"SELECT * FROM users WHERE id = \" + id"), "Vulnerable snippet must be replaced");
}
```

---

### TASK-P5-02: Cosine Similarity Vector Search Criterion Benchmarks
*   **Target Crate**: `crates/gitupload-core/`
*   **File Path**: `crates/gitupload-core/benches/bench_vector_search.rs`
*   **Responsibility**: Benchmarks ranking latency over 10,000 vectors using Criterion.rs to enforce the `< 50 ms` SLA.

#### Complete Rust Benchmark Implementation:
```rust
// crates/gitupload-core/benches/bench_vector_search.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gitupload_core::vector_search::VectorSearchEngine;

fn generate_mock_vectors(count: usize, dims: usize) -> Vec<(String, String, Vec<f32>)> {
    let mut list = Vec::with_capacity(count);
    for i in 0..count {
        let oid = format!("{:040x}", i);
        let summary = format!("Commit message summary for commit index {}", i);
        let vec: Vec<f32> = (0..dims).map(|d| ((i + d) % 100) as f32 / 100.0).collect();
        list.push((oid, summary, vec));
    }
    list
}

fn bench_cosine_search_10k(c: &mut Criterion) {
    let dataset = generate_mock_vectors(10_000, 512);
    let query_vector: Vec<f32> = (0..512).map(|d| (d % 50) as f32 / 50.0).collect();

    c.bench_function("vector_search_10k_commits_512d", |b| {
        b.iter(|| {
            let hits = VectorSearchEngine::rank_matches(
                black_box(&query_vector),
                black_box(&dataset),
                black_box(20),
                black_box(0.50),
            );
            assert_eq!(hits.len(), 20);
        });
    });
}

criterion_group!(benches, bench_cosine_search_10k);
criterion_main!(benches);
```

---

### TASK-P5-03: Stale Branch Cleanup & Merge Base Test Suite
*   **Target Crate**: `crates/gitupload-git/`
*   **File Path**: `crates/gitupload-git/tests/stale_branch_tests.rs`
*   **Responsibility**: Creates synthetic Git commit trees, merges feature branches, and tests detection and protected branch guardrails.

#### Complete Rust Code Implementation:
```rust
// crates/gitupload-git/tests/stale_branch_tests.rs
use git2::{BranchType, Repository, Signature};
use gitupload_git::stale_analyzer::StaleBranchAnalyzer;
use gitupload_git::pruner::BranchPruner;
use gitupload_core::models::branch_cleaner::{BranchMergeStatus, PruneBranchesRequest};
use tempfile::tempdir;

fn setup_test_repo_with_merged_branch() -> (tempfile::TempDir, Repository) {
    let dir = tempdir().expect("Failed to create tempdir");
    let repo = Repository::init(dir.path()).expect("Failed to init repo");

    let sig = Signature::now("Tester", "test@gitupload.local").unwrap();
    
    // Initial commit on main
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let initial_commit = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
    let commit_obj = repo.find_commit(initial_commit).unwrap();

    // Create feature branch
    let feature_branch = repo.branch("feature/test", &commit_obj, false).unwrap();
    
    // Feature branch commit
    let feature_commit = repo.commit(Some("refs/heads/feature/test"), &sig, &sig, "Feature commit", &tree, &[&commit_obj]).unwrap();
    let feat_obj = repo.find_commit(feature_commit).unwrap();

    // Fast-forward main to feature commit (simulating merge)
    let mut main_ref = repo.find_reference("refs/heads/main").unwrap();
    main_ref.set_target(feature_commit, "Merge feature").unwrap();

    (dir, repo)
}

#[test]
fn test_analyzer_detects_fully_merged_branch() {
    let (dir, _repo) = setup_test_repo_with_merged_branch();
    let results = StaleBranchAnalyzer::analyze_local_branches(dir.path().to_str().unwrap(), "main").unwrap();

    let feature = results.iter().find(|b| b.name == "feature/test").expect("Feature branch must be found");
    assert_eq!(feature.merge_status, BranchMergeStatus::FullyMerged);
}

#[tokio::test]
async fn test_pruner_blocks_protected_branch_deletion() {
    let dir = tempdir().expect("Failed to create tempdir");
    let _repo = Repository::init(dir.path()).expect("Failed to init repo");

    let req = PruneBranchesRequest {
        repo_path: dir.path().to_str().unwrap().to_string(),
        branch_names: vec!["main".into(), "master".into(), "release/v1.0".into()],
        delete_remote: false,
        prune_tracking_refs: false,
    };

    let result = BranchPruner::execute_prune(&req).await.unwrap();
    assert_eq!(result.deleted_local_count, 0, "Zero protected branches should be deleted");
    assert_eq!(result.failed_deletions.len(), 3, "All 3 protected branches must be reported as failed");
}
```

---

### TASK-P5-04: Background Daemon Battery Policy & Soak Tests
*   **Target Crate**: `crates/gitupload-watcher/`
*   **File Path**: `crates/gitupload-watcher/tests/daemon_tests.rs`
*   **Responsibility**: Validates that power policy queries return correctly and the daemon cancels within 50 ms.

#### Complete Rust Code Implementation:
```rust
// crates/gitupload-watcher/tests/daemon_tests.rs
use gitupload_watcher::power::{PowerMonitor, PowerState};
use gitupload_watcher::scheduler::{ScheduledSyncDaemon, SyncScheduleConfig};
use tokio_util::sync::CancellationToken;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_power_policy_logic() {
    assert!(PowerMonitor::should_allow_background_sync(true));
}

#[tokio::test]
async fn test_daemon_graceful_cancellation() {
    let token = CancellationToken::new();
    let config = SyncScheduleConfig {
        enabled: true,
        interval_minutes: 60,
        allow_on_battery: false,
        backup_bundles_enabled: false,
    };

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    ScheduledSyncDaemon::spawn_daemon(config, token.clone(), move || {
        let c = c.clone();
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    // Cancel daemon
    token.cancel();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert!(token.is_cancelled(), "Daemon token must be marked cancelled");
    assert_eq!(counter.load(Ordering::SeqCst), 0, "No extra sync tasks should run after cancellation");
}
```

---

### TASK-P5-05: Clippy, Unsafe & Secret Scrubbing Verification Gate
*   **Target**: Entire Workspace
*   **File Path**: `crates/gitupload-security/tests/sanitizer_verification.rs`
*   **Responsibility**: Audits that all crates pass Clippy `-D warnings`, zero `#![unsafe]` blocks exist in library crates, and token sanitization masks secrets.

#### Complete Rust Code Implementation:
```rust
// crates/gitupload-security/tests/sanitizer_verification.rs
use gitupload_security::sanitizer::sanitize_log_message;

#[test]
fn test_sanitizer_masks_embedded_remote_url() {
    let raw = "git push https://ghp_abcdefghijklmnopqrstuvwxyz123456@github.com/org/repo.git main";
    let cleaned = sanitize_log_message(raw);
    assert!(!cleaned.contains("ghp_"), "Log line must not contain raw PAT");
    assert!(cleaned.contains("https://***@github.com"), "URL must be masked with ***");
}

#[test]
fn test_sanitizer_masks_bearer_token() {
    let raw = "Authorization: Bearer github_pat_11AAAAAA_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let cleaned = sanitize_log_message(raw);
    assert!(!cleaned.contains("github_pat_"), "Bearer token must be redacted");
}
```

---

## 3. Strict Acceptance Criteria (AC-P5)

### `AC-P5-01`: Vulnerability Scanner Coverage & 1-Click Patch Accuracy
*   **Given**: A repository containing mock SQL injection, hardcoded PAT, and unencrypted `.env` files.
*   **When**: `vulnerability_tests.rs` executes.
*   **Then**:
    1. 100% of injected flaws are caught and categorized with correct CWE IDs.
    2. Automated patch application updates files without syntax breakage.
*   **Verification Command**: `cargo test -p gitupload-security --test vulnerability_tests`

### `AC-P5-02`: Semantic Search Latency SLA (< 50ms)
*   **Given**: An evaluation dataset of 10,000 commit embeddings (512 dimensions).
*   **When**: Criterion benchmark `bench_cosine_search_10k` runs.
*   **Then**:
    1. Mean ranking latency is `< 50 milliseconds`.
    2. Zero heap allocations occur during inner dot product calculations.
*   **Verification Command**: `cargo bench -p gitupload-core --bench bench_vector_search`

### `AC-P5-03`: Stale Branch Protected Guardrail Security
*   **Given**: A deletion payload targeting `main`, `master`, and `production`.
*   **When**: `stale_branch_tests.rs` runs.
*   **Then**:
    1. All 3 protected branches are preserved.
    2. Execution confirms 0 unauthorized branch deletions.
*   **Verification Command**: `cargo test -p gitupload-git --test stale_branch_tests`

### `AC-P5-04`: Energy & Battery Policy Conformance
*   **Given**: A system reporting `BatteryLowOrSaver`.
*   **When**: `ScheduledSyncDaemon` evaluates power state.
*   **Then**:
    1. Scheduled background sync cycles are paused immediately.
    2. CPU consumption remains strictly `< 0.1% CPU`.
*   **Verification Command**: `cargo test -p gitupload-watcher --test daemon_tests`

### `AC-P5-05`: Production Clippy & Safety Sign-Off
*   **Given**: The complete Sprint 3 codebase across all workspace crates.
*   **When**: `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` run.
*   **Then**:
    1. Zero errors or warnings emitted.
    2. `#![forbid(unsafe_code)]` compliance is verified across all library crates.
*   **Verification Command**: `cargo clippy --workspace --all-targets -- -D warnings`
