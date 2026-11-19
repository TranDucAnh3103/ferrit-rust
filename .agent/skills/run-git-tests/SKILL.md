---
name: run-git-tests
description: Executes the Rust integration test suite for Git operations and native hooks against isolated ephemeral repositories, validating libgit2, DAG revwalk, and conventional commit linters.
---

# Skill: run-git-tests

## Overview
This skill guides agents in running the Git operations integration test suite for `rust-gitupload`. It ensures all operations (`libgit2`, Git CLI fallback, DAG revwalk, and native hooks) are thoroughly verified against disposable, isolated test fixtures without altering local user repositories.

## Execution Steps

### 1. Safety Verification
Before executing any test command, ensure that:
- Tests strictly use `tempfile::tempdir()` for filesystem fixtures.
- No test targets existing repositories on `C:\` or `F:\`.
- Environment variable `GIT_TERMINAL_PROMPT=0` is set to prevent CLI hangs.

### 2. Run In-Process Git Engine Tests
Execute unit and integration tests for `gitupload-git`:
```powershell
cargo test -p gitupload-git -- --nocapture
```
This verifies:
- In-process repository initialization (`git_init`)
- Staging blobs and status detection (`git_status`, `git_add`)
- Tree construction and commit generation (< 3ms)
- Zero-copy diff inspection and patch generation
- DAG revwalk topology, swimlane assignment, and merge edge routing

### 3. Run Native Git Hooks Engine Tests
Execute hook validation tests for `gitupload-hooks`:
```powershell
cargo test -p gitupload-hooks -- --nocapture
```
This verifies:
- Conventional Commits 1.0.0 message validation (valid types, length limits, scope rules)
- Staged secret detection (blocking unencrypted `.env` files and high-entropy API tokens)
- File size boundary guards (> 50MB warnings)
- Pre-push branch protection simulation

### 4. Run Concurrent Stress & Cancellation Tests
Run multi-threaded cancellation tests:
```powershell
cargo test -p gitupload-git --test cancellation_tests -- --nocapture
```
Verifies that:
- Triggering `CancellationToken::cancel()` immediately stops network transfers in `libgit2` callbacks.
- Subprocess runners bound to Windows Job Objects exit cleanly without leaving orphaned `git-remote-https.exe` processes.

### 5. Report Findings
Summarize the test results:
- Number of tests passed, failed, and ignored.
- Execution latency for commit creation and revwalk.
- Confirmation of zero residual temp folders.
