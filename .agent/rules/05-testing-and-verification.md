# Rule 05: Testing, Benchmarks & Verification Protocols
**Scope:** Unit Tests, Integration Tests, Ephemeral Git Fixtures, Benchmarks  
**Severity:** High  

---

## 1. Testing Pyramid & Test Types

1. **Unit Tests (In-Crate)**:
   - Each crate must include unit tests in `src/` or `tests/` testing pure functions, regex parsers, conventional commit validators, and schema migrations.
   - Run: `cargo test --workspace --lib`

2. **Integration Tests with Ephemeral Repositories**:
   - Git operations (`gitupload-git`, `gitupload-hooks`) must be tested against real local repositories created in temporary folders (`tempfile::TempDir`).
   - Never run integration tests against the host machine's actual working git repositories.
   - Example fixture:
     ```rust
     fn create_ephemeral_repo() -> (tempfile::TempDir, git2::Repository) {
         let dir = tempfile::tempdir().expect("Failed to create tempdir");
         let repo = git2::Repository::init(dir.path()).expect("Failed to init git repo");
         (dir, repo)
     }
     ```

3. **External Service Mocking**:
   - HTTP calls to GitHub API and Cloud AI providers must use `wiremock` in test environments to ensure tests run offline, deterministically, and fast.
   - Credential vault operations must use a mock in-memory keyring during CI runs.

---

## 2. Performance & Benchmark SLAs

All code modifications must maintain the core performance SLAs defined in [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md):

| Benchmark Metric | Target SLA | Verification Tool |
| :--- | :--- | :--- |
| **Idle Memory Footprint** | `< 28 MB RAM` | Sysinternals VMMap / Criterion benchmark |
| **Active Load Memory** | `< 75 MB RAM` | Task Manager Working Set under load |
| **Cold Startup Latency** | `< 100 ms` | Tracing timestamp startup telemetry |
| **In-Process Commit Time** | `< 3 ms` | Criterion microbenchmark |
| **DAG Revwalk (10,000 commits)** | `< 15 ms` | Criterion microbenchmark |
| **Native Hook Execution** | `< 3 ms` | Criterion microbenchmark |

Run benchmarks using:
```bash
cargo bench --workspace
```

---

## 3. Pre-Commit Quality Checklist

Before submitting code or reporting completion:
- [ ] `cargo check --workspace` passes with zero errors.
- [ ] `cargo test --workspace` passes with 100% test success.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` emits zero warnings.
- [ ] `cargo fmt --check` passes cleanly.
- [ ] No plaintext secrets or unredacted tokens exist in logs or code.
- [ ] All commits follow Conventional Commits 1.0.0 format.
