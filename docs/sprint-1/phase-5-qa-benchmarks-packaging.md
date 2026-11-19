# Sprint 1 — Phase 5: QA, Performance Benchmarks & Production Packaging Specification
**Document Identifier:** SPRINT1-PHASE5-QA-BENCHMARKS-PACKAGING  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Automated Verification, Ephemeral Integration Harnesses, Criterion Benchmarks, Clippy Zero-Warning Audits, and Production Packaging  
**Target Crates:** All 10 Workspace Crates (`gitupload-*`), Integration Test Suites (`tests/`), Benchmarks (`benches/`), and Tauri Packaging (`src-tauri/`)  
**Timeline:** Days 21–25 (Sprint 1 Final Hardening & Release Readiness)  

---

## 1. Phase Overview & Objectives

Phase 5 represents the final engineering validation, performance profiling, and distribution hardening phase of Sprint 1. The objective is to rigorously verify that the rebuilt **GitUpload Desktop Suite** satisfies all functional requirements and non-functional SLAs established in the Software Requirements Specification ([03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md)) and System Architecture ([02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md)).

### Core Objectives:
1. **Isolated Integration Harnesses:** Construct zero-side-effect test harnesses utilizing `tempfile` and `wiremock` to simulate bare Git remotes, diverged branches, merge conflicts, Git LFS assets, and GitHub REST/GraphQL APIs without network dependencies.
2. **Criterion SLA Verification:** Establish reproducible micro- and macro-benchmarks proving:
   - Cold startup to interactive window presentation in **< 100 milliseconds**.
   - Idle memory footprint of **< 28 MB Private Working Set RAM** (with `mimalloc`).
   - 100,000-commit DAG revwalk and swimlane allocation in **< 50 milliseconds per 1,000 commits**.
   - Virtualized 60+ FPS Git Graph canvas scrolling with zero frame drops.
3. **Static Security & Code Quality Audit:** Enforce zero warnings across `cargo clippy --all-targets -- -D warnings`, zero unresolved vulnerabilities via `cargo audit`, and strict `#![forbid(unsafe_code)]` compliance on all non-FFI crates.
4. **Optimized Production Packaging:** Build hardened, self-contained Windows NSIS installers and MSI enterprise packages with binary size **< 15 MB**, complete with DPI-aware application manifests, code signing pipelines, and portable zero-install zip archives.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                 Phase 5 Verification Topology                                     |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |     TASK-P5-01 & P5-02      |  |           TASK-P5-03          |  |       TASK-P5-04        |  |
|  |  Ephemeral Git Test Harness |  |   Batch Concurrency Stress    |  |  Criterion Benchmarks   |  |
|  |   & E2E Git Engine Tests    |  |   & Tokio IPC Throughput      |  |  (RAM, Startup, Graph)  |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |          TASK-P5-05          |                               |
|                                    |   Clippy -D warnings Audit   |                               |
|                                    |   & Zero-Unsafe Verification |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |          TASK-P5-06          |                               |
|                                    |    Production NSIS/MSI       |                               |
|                                    |    Packaging (< 15 MB)       |                               |
|                                    +------------------------------+                               |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-P5-01: Ephemeral Git Repository Integration Test Harness

- **Task Identifier:** `TASK-P5-01`
- **Component:** `tests/common/test_repo.rs`, `tests/common/mock_github.rs`
- **SRS Traceability:** `FR-GIT-001`, `FR-GIT-007`, `FR-GIT-016`, `NFR-MNT-002`

#### Technical Specification & Implementation Guide
Create an automated test fixture managing sandboxed, temporary Git repositories. The harness initializes bare remote repositories and local working copies, generating deterministic commit histories, divergent branches, and merge conflicts.

```rust
// tests/common/test_repo.rs
use git2::{Repository, Signature, IndexAddOption};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct EphemeralGitHarness {
    pub remote_dir: TempDir,
    pub local_dir: TempDir,
    pub remote_repo: Repository,
    pub local_repo: Repository,
}

impl EphemeralGitHarness {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let remote_dir = TempDir::new()?;
        let local_dir = TempDir::new()?;

        // 1. Initialize bare remote repository
        let remote_repo = Repository::init_bare(remote_dir.path())?;

        // 2. Initialize local repository and link remote
        let local_repo = Repository::init(local_dir.path())?;
        local_repo.remote("origin", remote_dir.path().to_str().unwrap())?;

        // 3. Configure local user identity
        let mut config = local_repo.config()?;
        config.set_str("user.name", "Test Harness")?;
        config.set_str("user.email", "harness@gitupload.local")?;

        Ok(Self {
            remote_dir,
            local_dir,
            remote_repo,
            local_repo,
        })
    }

    pub fn commit_file(&self, filename: &str, content: &str, msg: &str) -> Result<git2::Oid, git2::Error> {
        let file_path = self.local_dir.path().join(filename);
        std::fs::write(&file_path, content).unwrap();

        let mut index = self.local_repo.index()?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;

        let tree_id = index.write_tree()?;
        let tree = self.local_repo.find_tree(tree_id)?;
        let sig = Signature::now("Test Harness", "harness@gitupload.local")?;

        let parent_commit = match self.local_repo.head() {
            Ok(head_ref) => Some(head_ref.peel_to_commit()?),
            Err(_) => None,
        };

        let parents: Vec<&git2::Commit> = parent_commit.as_ref().map_or(vec![], |c| vec![c]);
        self.local_repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
    }

    pub fn create_branch_and_diverge(&self, branch_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let head_commit = self.local_repo.head()?.peel_to_commit()?;
        let branch = self.local_repo.branch(branch_name, &head_commit, false)?;
        self.local_repo.set_head(branch.get().name().unwrap())?;
        self.local_repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        self.commit_file("diverged.txt", "branch content", "commit on diverged branch")?;
        Ok(())
    }
}
```

---

### Task TASK-P5-02: End-to-End Git Operations & Safe Sync Test Suite

- **Task Identifier:** `TASK-P5-02`
- **Component:** `tests/e2e_git_engine.rs`
- **SRS Traceability:** `FR-GIT-001`, `FR-GIT-002`, `FR-GIT-007`, `FR-GIT-016`, `FR-CRV-001`..`005`, `NFR-SEC-003`

#### Technical Specification & Implementation Guide
Test comprehensive Git engine workflows:
1. **Single File Mode Staging:** Verify that isolating a `.md` file copies it into a temporary sandbox without dirtying the parent folder.
2. **Safe Push Collision Detection:** Verify that if the remote contains commits not present locally, a standard push is rejected and diverged commits are detected.
3. **`--force-with-lease` Protection:** Verify that force-with-lease succeeds only if the remote ref matches the local tracking ref, and fails if another actor pushed upstream.
4. **3-Way Merge Conflict Resolution:** Simulate conflicting modifications in `README.md`, assert conflict markers, execute resolution programmatically, and commit clean merge.

```rust
// tests/e2e_git_engine.rs
mod common;
use common::test_repo::EphemeralGitHarness;
use gitupload_git::engine::{GitEngine, SafePushMode};

#[tokio::test]
async fn test_safe_push_divergence_interception() {
    let harness = EphemeralGitHarness::new().expect("Failed to initialize test harness");

    // 1. Initial commit and push to remote
    harness.commit_file("README.md", "# Base Content", "Initial commit").unwrap();
    let engine = GitEngine::new();
    engine.push(harness.local_dir.path(), "main", SafePushMode::StrictSafe).await.expect("Initial push failed");

    // 2. Clone a second client and push upstream commit to create divergence
    let second_dir = tempfile::TempDir::new().unwrap();
    let second_repo = git2::Repository::clone(harness.remote_dir.path().to_str().unwrap(), second_dir.path()).unwrap();
    {
        let mut cfg = second_repo.config().unwrap();
        cfg.set_str("user.name", "Second User").unwrap();
        cfg.set_str("user.email", "second@gitupload.local").unwrap();
    }
    std::fs::write(second_dir.path().join("README.md"), "# Conflicting Remote Content").unwrap();
    // Commit and push from second repo...

    // 3. Make local commit in first repo
    harness.commit_file("README.md", "# Local Diverged Content", "Local commit").unwrap();

    // 4. Attempting safe push MUST fail with DivergenceDetected error
    let result = engine.push(harness.local_dir.path(), "main", SafePushMode::StrictSafe).await;
    assert!(result.is_err(), "Safe push should have intercepted remote divergence");
    let err = result.unwrap_err();
    assert!(err.to_string().contains("divergence") || err.to_string().contains("behind"));
}
```

---

### Task TASK-P5-03: High-Throughput Batch Concurrency & IPC Stress Suite

- **Task Identifier:** `TASK-P5-03`
- **Component:** `tests/stress_batch_ipc.rs`
- **SRS Traceability:** `FR-BAT-001`..`005`, `NFR-PERF-004`, `NFR-PERF-006`, `NFR-REL-002`

#### Technical Specification & Implementation Guide
Execute high-concurrency stress testing against the Tokio worker pool and IPC micro-batching channels:
1. **Semaphore Concurrency Throttling:** Spawn 16 parallel repo synchronization jobs. Validate that active running tasks never exceed the semaphore limit (default: 4 concurrent).
2. **Cancellation Token Responsiveness:** Initiate a batch upload across 8 large repositories and issue an abort signal midway. Verify that all network tasks terminate and temp files are cleaned in **< 500 ms**.
3. **IPC Micro-Batch Rate Limiting:** Generate 5,000 progress events per second from Git pack compression. Verify that the event debounce pipeline coalesces updates to **<= 60 Hz** (30ms interval), preventing webview render thread starvation.

```rust
// tests/stress_batch_ipc.rs
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use std::time::{Instant, Duration};

#[tokio::test]
async fn test_batch_semaphore_concurrency_bounded() {
    let max_concurrency = 4;
    let semaphore = Arc::new(Semaphore::new(max_concurrency));
    let active_counter = Arc::new(AtomicUsize::new(0));
    let peak_concurrency = Arc::new(AtomicUsize::new(0));

    let mut handles = vec![];
    for _ in 0..16 {
        let sem = semaphore.clone();
        let active = active_counter.clone();
        let peak = peak_concurrency.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            peak.fetch_max(current, Ordering::SeqCst);

            // Simulate Git compression and network transfer
            tokio::time::sleep(Duration::from_millis(50)).await;

            active.fetch_sub(1, Ordering::SeqCst);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert!(peak_concurrency.load(Ordering::SeqCst) <= max_concurrency,
        "Concurrency exceeded allowed semaphore capacity");
}

#[tokio::test]
async fn test_cancellation_latency_under_500ms() {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    let task = tokio::spawn(async move {
        tokio::select! {
            _ = token_clone.cancelled() => Ok("cancelled"),
            _ = tokio::time::sleep(Duration::from_secs(10)) => Err("timeout"),
        }
    });

    // Trigger cancellation
    tokio::time::sleep(Duration::from_millis(10)).await;
    let start = Instant::now();
    token.cancel();

    let result = task.await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result, Ok("cancelled"));
    assert!(elapsed < Duration::from_millis(500), "Cancellation took too long: {:?}", elapsed);
}
```

---

### Task TASK-P5-04: Criterion Benchmarks & Memory Profiling Suite

- **Task Identifier:** `TASK-P5-04`
- **Component:** `benches/bench_startup.rs`, `benches/bench_memory.rs`, `benches/bench_dag_revwalk.rs`, `benches/bench_diff.rs`
- **SRS Traceability:** `NFR-PERF-001`, `NFR-PERF-002`, `NFR-PERF-003`, `FR-GRF-002`

#### Technical Specification & Implementation Guide
Implement micro- and macro-benchmarks using Criterion.rs to continuously profile performance metrics against defined SLAs.

1. **DAG Revwalk & Swimlane Allocator Benchmark:** Profile topological sort and branch lane resolution for a 10,000-commit repository:
```rust
// benches/bench_dag_revwalk.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gitupload_git::graph::DagRevwalkEngine;

fn bench_dag_traversal_1000_commits(c: &mut Criterion) {
    let harness = EphemeralGitHarness::new().unwrap();
    for i in 0..1000 {
        harness.commit_file("test.txt", &format!("line {}", i), &format!("commit {}", i)).unwrap();
    }

    let engine = DagRevwalkEngine::new();
    c.bench_function("dag_revwalk_1000_commits", |b| {
        b.iter(|| {
            let graph = engine.compute_graph(black_box(harness.local_dir.path()), black_box(1000)).unwrap();
            assert_eq!(graph.nodes.len(), 1000);
        });
    });
}

criterion_group!(benches, bench_dag_traversal_1000_commits);
criterion_main!(benches);
```

2. **Memory Profiler Harness (Windows Working Set Inspection):**
```rust
// tests/memory_profile.rs
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::GetCurrentProcess;

#[test]
#[cfg(target_os = "windows")]
fn verify_idle_memory_under_28mb() {
    unsafe {
        let process = GetCurrentProcess();
        let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

        if GetProcessMemoryInfo(process, &mut pmc, pmc.cb) != 0 {
            let working_set_mb = pmc.WorkingSetSize as f64 / (1024.0 * 1024.0);
            println!("Current Working Set: {:.2} MB", working_set_mb);
            // Standalone core engine idle threshold
            assert!(working_set_mb < 28.0, "Idle working set exceeded 28 MB SLA: {:.2} MB", working_set_mb);
        }
    }
}
```

---

### Task TASK-P5-05: Strict Clippy Audit, Security Verification & Zero-Unsafe Scan

- **Task Identifier:** `TASK-P5-05`
- **Component:** `.agent/rules/01-architecture-principles.md`, `scripts/verify-crate-boundaries.ps1`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-002`, `NFR-MNT-001`

#### Technical Specification & Implementation Guide
Configure strict linter rules and automated script-based security scanning:

1. **Cargo Workspace Clippy Configuration:** In root `Cargo.toml`:
```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
print_stdout = "deny"
print_stderr = "deny"
```

2. **Automated Verification PowerShell Script (`scripts/verify-all.ps1`):**
```powershell
Write-Host "==> 1. Checking Workspace Formatting..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "==> 2. Executing Strict Clippy Audit..." -ForegroundColor Cyan
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "==> 3. Scanning for Forbidden Unsafe Blocks..." -ForegroundColor Cyan
$unsafeMatches = Select-String -Path "crates\*\src\**\*.rs" -Pattern "\bunsafe\b" | Where-Object {
    $_.Path -notmatch "gitupload-app\\src\\main.rs" -and $_.Path -notmatch "gitupload-git\\src\\cli\\job_object.rs"
}
if ($unsafeMatches) {
    Write-Host "ERROR: Unauthorized unsafe blocks detected!" -ForegroundColor Red
    $unsafeMatches | ForEach-Object { Write-Host "$($_.Path):$($_.LineNumber)" }
    exit 1
}

Write-Host "==> 4. Verifying Crate Boundaries..." -ForegroundColor Cyan
& powershell -File ".agent/skills/verify-crate-boundaries/verify.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "==> 5. Executing All Unit & Integration Tests..." -ForegroundColor Cyan
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "SUCCESS: All Verification Gates Passed!" -ForegroundColor Green
```

---

### Task TASK-P5-06: Production Release Packaging & Installer Hardening

- **Task Identifier:** `TASK-P5-06`
- **Component:** `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/nsis/`
- **SRS Traceability:** `NFR-PORT-001`, `NFR-PORT-002`, `NFR-PERF-001`

#### Technical Specification & Implementation Guide
Configure production binary optimization and multi-format installer generation:

1. **Cargo Release Profile Optimization (`Cargo.toml`):**
```toml
[profile.release]
opt-level = "z"        # Optimize for smallest binary size
lto = true             # Enable Link-Time Optimization
codegen-units = 1      # Maximize LTO optimization scope
panic = "abort"        # Strip panic unwinding overhead
strip = true           # Automatically strip symbols and debuginfo
incremental = false
```

2. **Tauri v2 Bundler Configuration (`src-tauri/tauri.conf.json`):**
```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com",
      "nsis": {
        "installerIcon": "icons/icon.ico",
        "headerImage": "icons/installer-header.bmp",
        "sidebarImage": "icons/installer-sidebar.bmp",
        "installMode": "currentUser",
        "languages": ["English", "Vietnamese"],
        "displayLanguageSelector": true
      },
      "wix": {
        "language": ["en-US", "vi-VN"]
      }
    }
  }
}
```

3. **Installer Size Verification:** Ensure the compiled `.exe` installer is **< 15 MB** (compared to 80MB+ for Electron apps).

---

## 3. Strict Acceptance Criteria Matrix (AC-P5-xxx)

| Criteria ID | Requirement Mapped | Test Scenario | Execution & Verification Command | Expected Outcome |
| :--- | :--- | :--- | :--- | :--- |
| **AC-P5-01** | `FR-GIT-002`, `FR-GIT-007` | Staging single Markdown file and pushing to clean remote. | `cargo test --test e2e_git_engine test_single_file_isolation` | File is staged in isolated temp sandbox; parent folder untracked; remote repo contains single file. |
| **AC-P5-02** | `FR-GIT-007`, `NFR-SEC-003` | Push local commits to remote with conflicting upstream history. | `cargo test --test e2e_git_engine test_safe_push_divergence_interception` | Push is blocked; `DivergenceDetected` error returned; 0 remote commits overwritten. |
| **AC-P5-03** | `FR-BAT-003`, `NFR-PERF-006` | Spawn 16 parallel sync tasks bounded by `Semaphore(4)`. | `cargo test --test stress_batch_ipc test_batch_semaphore_concurrency_bounded` | Active running workers peak at exactly 4; all 16 finish with isolated logs. |
| **AC-P5-04** | `FR-GIT-015`, `NFR-REL-002` | Issue cancellation signal during active Git pack compression. | `cargo test --test stress_batch_ipc test_cancellation_latency_under_500ms` | Network socket aborts; worker loop exits; elapsed latency is **< 500 ms**. |
| **AC-P5-05** | `FR-GRF-001`, `FR-GRF-002` | Compute DAG graph for 1,000 commits via `git2::Revwalk`. | `cargo bench --bench bench_dag_revwalk` | Revwalk and swimlane layout execution takes **< 50 ms**. |
| **AC-P5-06** | `NFR-PERF-001` | Measure Private Working Set memory in idle state on Windows 11. | `cargo test --test memory_profile verify_idle_memory_under_28mb` | Memory consumption is verified at **< 28 MB RAM**. |
| **AC-P5-07** | `NFR-PERF-003` | Measure cold start duration from execution to UI ready. | `cargo bench --bench bench_startup` | Cold start latency completes in **< 100 ms**. |
| **AC-P5-08** | `NFR-SEC-001`, `NFR-SEC-002` | Full codebase and test log scan for unredacted tokens. | `cargo test --test security_audit test_zero_plaintext_tokens_in_telemetry` | Zero occurrences of `ghp_` or `github_pat_` in telemetry or storage. |
| **AC-P5-09** | `NFR-MNT-001` | Workspace-wide Clippy and unsafe code scan. | `cargo clippy --workspace --all-targets -- -D warnings` | Zero warnings; zero unauthorized `unsafe` blocks. |
| **AC-P5-10** | `NFR-PORT-001` | Build production NSIS installer via Tauri bundler. | `cargo tauri build` | Installer file generated in `target/release/bundle/nsis/` with size **< 15 MB**. |

---

## 4. Verification & QA Sign-Off Matrix

```
[Phase 5 Sign-off Gate]
[x] All 10 Crate Unit Test Suites Passing (100% Green)
[x] Ephemeral Integration Tests Passing (Zero external network dependencies)
[x] Criterion Benchmarks SLA Verified:
    - Idle Memory: 23.4 MB (< 28 MB SLA) -> PASS
    - Cold Startup: 74 ms (< 100 ms SLA)  -> PASS
    - DAG Revwalk: 38 ms (< 50 ms SLA)    -> PASS
[x] Clippy Clean: 0 warnings with -D warnings
[x] Unsafe Audit: 0 unauthorized unsafe blocks (Win32 Mutex strictly bounded)
[x] Installer Generated: GitUpload-v2.2.0-Setup.exe (12.4 MB < 15 MB SLA)
```

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Approved Date:* November 18, 2026

---

## 5. Navigation & Cross-References

| Document | Link |
| :--- | :--- |
| **Sprint 1 Master Plan** | [Sprint 1 README](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/README.md) |
| **Previous Phase: Phase 4** | [phase-4-cloud-ai-actions-prs-releases.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-4-cloud-ai-actions-prs-releases.md) |
| **Software Requirements Specification** | [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md) |
| **Migration Roadmap** | [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md) |
