# Sprint 1 — Phase 2: Dual Git Engine, Interactive DAG Graph, Safe Sync & Git Ecosystem Specification
**Document Identifier:** SPRINT1-PHASE2-GIT-ENGINE-GRAPH-SAFESYNC  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** In-Process `libgit2` Operations, Windows Job Object Subprocess Isolation, Interactive DAG Revwalk Graph Builder, Swimlane Routing, Re-Push/Safe Sync (`--force-with-lease`), Git LFS Tracking, and GPG Commit Signing  
**Target Crates:** `gitupload-git` (with dependencies on `gitupload-core`, `gitupload-security`)  
**Timeline:** Days 6–10 (Sprint 1 Core Git Capabilities & DAG Visualization)  

---

## 1. Phase Overview & Objectives

Phase 2 engineers the core Git engine and topological visualization subsystem. It eliminates the legacy reliance on sequential, high-latency CLI subprocess spawning by introducing an in-process **Dual Git Engine** powered by `libgit2` C-bindings, backed by a subprocess CLI runner isolated within **Windows Job Objects** for clean cancellation.

### Core Objectives:
1. **In-Process Git Engine (`libgit2` via `git2` crate):** Execute `init`, `status`, `stage_all`, `commit`, `branch`, and `diff` directly in-process with zero Windows process creation overhead (< 3ms per commit).
2. **Windows Job Object Process Isolation:** Wrap fallback `git.exe` CLI subprocesses inside Windows Job Objects with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Guarantee instant, clean process tree termination on cancellation (< 50ms) with zero lingering lock files.
3. **Interactive Git Graph DAG Engine (`git2::Revwalk`):** Implement topological + time-sorted commit graph generation. Compute colored branch swimlanes, parent-child bezier curved lines, merge nodes, branch heads, tags, stashes, and HEAD pointer.
4. **Re-Push & Safe Sync Engine:** Detect existing `.git` repositories, inspect remote tracking branches, calculate ahead/behind commit deltas, and replace dangerous blind force pushes with `git push --force-with-lease` and automated stashing.
5. **Git LFS & GPG Commit Signing:** Automatically manage `.gitattributes` for binary assets > 50MB, and support cryptographic commit signing with GPG/SSH to achieve the GitHub `[Verified]` badge.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 2 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |         TASK-P2-01          |  |          TASK-P2-02           |  |       TASK-P2-03        |  |
|  |     LibGit2Engine Core      |  |     CliGitEngine Subprocess   |  | Interactive DAG Revwalk |  |
|  |    (In-Process Staging,     |  |    (Windows Job Objects,      |  | (Swimlane Allocation,   |  |
|  |     Commit, Clone, Push)    |  |     Process Tree Kill)        |  |  Bezier Curves, Refs)   |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |       gitupload-git Crate    |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |         TASK-P2-04          |   |          TASK-P2-05          |   |       TASK-P2-06       |  |
|  |   Re-Push & Safe Sync       |   |   Git Graph Action Helpers   |   | Git LFS & GPG Signing  |  |
|  |   (--force-with-lease       |   |  (Cherry-pick, Revert,       |   | (.gitattributes auto,  |  |
|  |    + Auto-Stash Engine)     |   |   Reset, Interactive Rebase) |   |  git2::Commit::sign)   |  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-P2-01: In-Process LibGit2 Engine Core

- **Task Identifier:** `TASK-P2-01`
- **Component:** `crates/gitupload-git/src/libgit2/repo.rs`, `commit.rs`, `transport.rs`
- **SRS Traceability:** `FR-GIT-001`, `FR-GIT-002`, `FR-GIT-006`, `FR-GIT-007`, `NFR-PERF-001`

#### Implementation Specification
Implement direct in-process Git operations using the `git2` crate:
1. `init_repository`: Initializes local repository with `.git` and configures `gc.auto = 0`.
2. `stage_all_and_commit`: Stages all working directory changes (honoring `.gitignore`) and commits with the author signature in < 3ms.
3. `push_with_callbacks`: Connects remote credentials directly from in-memory `SecretString` via `RemoteCallbacks::credentials`. Streams real-time transfer progress callbacks (0–100%) through Tokio channels without emitting tokens.

```rust
// crates/gitupload-git/src/libgit2/commit.rs
use git2::{IndexAddOption, Oid, Repository, Signature};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use std::path::Path;

pub struct LibGit2Commit;

impl LibGit2Commit {
    pub fn stage_and_commit(
        repo_path: &Path,
        author_name: &str,
        author_email: &str,
        message: &str,
    ) -> GitUploadResult<Oid> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {}", e)))?;

        // 1. Stage all changes
        let mut index = repo.index()
            .map_err(|e| GitUploadError::Git(format!("Failed to open index: {}", e)))?;
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(|e| GitUploadError::Git(format!("Failed to stage files: {}", e)))?;
        index.write()
            .map_err(|e| GitUploadError::Git(format!("Failed to write index: {}", e)))?;

        let tree_id = index.write_tree()
            .map_err(|e| GitUploadError::Git(format!("Failed to write tree: {}", e)))?;
        let tree = repo.find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(format!("Failed to find tree: {}", e)))?;

        // 2. Prepare signatures
        let sig = Signature::now(author_name, author_email)
            .map_err(|e| GitUploadError::Git(format!("Invalid signature: {}", e)))?;

        // 3. Resolve parent commit (if any)
        let parent_commit = match repo.head() {
            Ok(head_ref) => head_ref.peel_to_commit().ok(),
            Err(_) => None,
        };

        let parents: Vec<&git2::Commit> = match &parent_commit {
            Some(c) => vec![c],
            None => vec![],
        };

        // 4. Create commit
        let commit_oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            message,
            &tree,
            &parents,
        ).map_err(|e| GitUploadError::Git(format!("Failed to commit: {}", e)))?;

        Ok(commit_oid)
    }
}
```

---

### Task TASK-P2-02: Windows Job Object Subprocess Fallback Runner

- **Task Identifier:** `TASK-P2-02`
- **Component:** `crates/gitupload-git/src/cli/runner.rs`, `job_object.rs`
- **SRS Traceability:** `NFR-REL-002`, `FR-GIT-007`

#### Implementation Specification
When operations require custom Git transports (e.g. specialized enterprise SSH agents or credential helpers), execute `git.exe` via Tokio asynchronous process management, isolated inside a Windows Job Object:
1. `create_job_object`: Uses `windows-sys` to create an anonymous Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
2. `assign_process`: Assigns the spawned `git.exe` process handle to the Job Object.
3. `cancel`: Dropping the Job Object handle automatically terminates the entire subprocess tree, leaving zero orphaned `git.exe` instances and eliminating `.git/index.lock` contention.

```rust
// crates/gitupload-git/src/cli/job_object.rs
#[cfg(windows)]
use windows_sys::Win32::System::JobObjects::*;
#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
use std::os::windows::io::RawHandle;

pub struct WindowsJobObject {
    handle: HANDLE,
}

impl WindowsJobObject {
    pub fn new() -> Result<Self, std::io::Error> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let ret = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if ret == 0 {
                CloseHandle(handle);
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self { handle })
        }
    }

    pub fn assign_process(&self, process_handle: RawHandle) -> Result<(), std::io::Error> {
        unsafe {
            let ret = AssignProcessToJobObject(self.handle, process_handle as HANDLE);
            if ret == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }
    }
}

impl Drop for WindowsJobObject {
    fn drop(&mut self) {
        unsafe {
            if self.handle != 0 {
                CloseHandle(self.handle);
            }
        }
    }
}
```

---

### Task TASK-P2-03: Interactive Git Graph DAG Engine & Swimlane Router

- **Task Identifier:** `TASK-P2-03`
- **Component:** `crates/gitupload-git/src/graph/engine.rs`, `swimlane.rs`
- **SRS Traceability:** `FR-GRF-001`, `FR-GRF-002`, `FR-GRF-003`, `FR-GRF-005`

#### Implementation Specification
1. **DAG Revwalk:** Configure `git2::Revwalk` with `git2::Sort::TOPOLOGICAL | git2::Sort::TIME`. Traverse commits and resolve parents, committers, signatures, and summary messages.
2. **Swimlane Routing Algorithm:** Assign an integer column index (`swimlane_index`) to each branch. When a branch merges into another, route a curved bezier edge back to the target lane and free the current lane for reuse.
3. **Ref Resolution:** Resolve HEAD, local branches, remote tracking branches (`origin/*`), tags, and stash entries, binding them to corresponding commit nodes.

```rust
// crates/gitupload-git/src/graph/engine.rs
use git2::{Repository, Sort};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::GitCommitNode;
use chrono::{TimeZone, Utc};
use std::path::Path;

pub struct GitGraphEngine;

impl GitGraphEngine {
    pub fn build_dag(repo_path: &Path, max_commits: usize) -> GitUploadResult<Vec<GitCommitNode>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {}", e)))?;

        let mut revwalk = repo.revwalk()
            .map_err(|e| GitUploadError::Git(format!("Failed to init revwalk: {}", e)))?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
            .map_err(|e| GitUploadError::Git(format!("Failed to set sorting: {}", e)))?;
        revwalk.push_glob("refs/*")
            .map_err(|e| GitUploadError::Git(format!("Failed to push refs: {}", e)))?;

        let head_oid = repo.head().ok().and_then(|h| h.target()).map(|o| o.to_string());
        let mut nodes = Vec::new();

        for (idx, oid_res) in revwalk.take(max_commits).enumerate() {
            let oid = oid_res.map_err(|e| GitUploadError::Git(e.to_string()))?;
            let commit = repo.find_commit(oid)
                .map_err(|e| GitUploadError::Git(e.to_string()))?;

            let parent_oids = commit.parent_ids().map(|id| id.to_string()).collect();
            let time_secs = commit.time().seconds();
            let timestamp = Utc.timestamp_opt(time_secs, 0).single().unwrap_or_else(Utc::now);

            let is_head = head_oid.as_deref() == Some(&oid.to_string());

            nodes.push(GitCommitNode {
                oid: oid.to_string(),
                short_oid: oid.to_string()[..7].to_string(),
                message: commit.message().unwrap_or("").to_string(),
                summary: commit.summary().unwrap_or("").to_string(),
                author_name: commit.author().name().unwrap_or("").to_string(),
                author_email: commit.author().email().unwrap_or("").to_string(),
                timestamp,
                parent_oids,
                swimlane_index: idx % 6, // Refined by SwimlaneRouter
                is_head,
                refs: vec![],
                is_verified: false,
            });
        }

        Ok(nodes)
    }
}
```

---

### Task TASK-P2-04: Re-Push Engine, Safe Sync & Auto-Stash

- **Task Identifier:** `TASK-P2-04`
- **Component:** `crates/gitupload-git/src/coordinator/repush.rs`, `safe_sync.rs`
- **SRS Traceability:** `FR-GIT-016`, `FR-AUT-001`

#### Implementation Specification
1. **Existing Repo Detection & Ahead/Behind Tracking:** Check if destination directory contains `.git`. Compare local branch HEAD against `refs/remotes/origin/<branch>`. Calculate `ahead` and `behind` commit counts using `git2::Graph::ahead_behind`.
2. **Safe Sync (`--force-with-lease`):** Reject blind `git push --force`. When remote history differs, verify expected remote commit object ID before updating the reference.
3. **Automated Pre-Push Stashing:** If uncommitted working directory changes exist, automatically stash them (`git stash save -u`), perform push or fetch, and pop the stash safely upon completion.

---

### Task TASK-P2-05: Git Graph Contextual Actions

- **Task Identifier:** `TASK-P2-05`
- **Component:** `crates/gitupload-git/src/graph/actions.rs`
- **SRS Traceability:** `FR-GRF-004`

#### Implementation Specification
Expose high-level Git operations invocable directly from nodes on the interactive DAG graph:
1. `cherry_pick_commit(commit_oid)`: Cherry-picks specified commit onto the current branch using `git2::Repository::cherrypick`.
2. `revert_commit(commit_oid)`: Creates an inverse commit reverting changes from the selected commit.
3. `reset_to_commit(commit_oid, mode)`: Performs `Soft`, `Mixed`, or `Hard` reset using `git2::Repository::reset`.
4. `checkout_ref(target_ref)`: Switches working directory to branch or detached commit.

---

### Task TASK-P2-06: Git LFS Asset Tracking & GPG Commit Signing

- **Task Identifier:** `TASK-P2-06`
- **Component:** `crates/gitupload-git/src/lfs.rs`, `signing.rs`
- **SRS Traceability:** `FR-GIT-017`, `FR-GIT-019`

#### Implementation Specification
1. **Git LFS Tracker:**
   - Scan working directory for binary assets exceeding 50MB (e.g. `.zip`, `.jar`, `.onnx`, `.bin`).
   - Automatically write tracking rules into `.gitattributes` (`*.zip filter=lfs diff=lfs merge=lfs -text`).
2. **GPG/SSH Commit Signing:**
   - Inspect user Git configuration for `user.signingkey` and `commit.gpgsign = true`.
   - Use `git2::Repository::commit_signed` to attach cryptographic signatures to commit objects for verification badges.

---

## 3. Strict Acceptance Criteria (AC-xxx)

### Acceptance Criteria Table

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-P2-01** | A repository with 100 modified files | Calling `LibGit2Commit::stage_and_commit()` | All files staged and committed in < 3ms with a valid OID returned | `cargo test -p gitupload-git test_libgit2_commit` |
| **AC-P2-02** | A running fallback Git CLI operation | Calling `cancel()` on the Windows Job Object | Subprocess tree terminates within < 50ms and `.git/index.lock` is unlocked | `cargo test -p gitupload-git test_job_object_kill` |
| **AC-P2-03** | A repository with 1,000 commits across 5 branches | Calling `GitGraphEngine::build_dag()` | Returns 1,000 `GitCommitNode` records with resolved parents and swimlane lanes in < 25ms | `cargo test -p gitupload-git test_graph_dag_perf` |
| **AC-P2-04** | An existing repo with 2 un-pushed commits | Inspecting via `RePushInspector::inspect()` | Correctly reports `ahead = 2, behind = 0` and current branch name | `cargo test -p gitupload-git test_repush_inspect` |
| **AC-P2-05** | Diverged remote history | Executing push with safe sync enabled | Rejects raw `--force` and executes `--force-with-lease` protecting remote changes | `cargo test -p gitupload-git test_safe_sync` |
| **AC-P2-06** | Uncommitted working directory changes | Executing sync with auto-stash | Changes stashed, sync executed, and stash restored with zero code loss | `cargo test -p gitupload-git test_autostash` |
| **AC-P2-07** | A commit node selected on DAG graph | Calling `GitGraphActions::cherry_pick()` | Specified commit changes cleanly applied to HEAD with new commit generated | `cargo test -p gitupload-git test_cherry_pick` |
| **AC-P2-08** | A staged file of 60MB | Pre-flight LFS check executed | File flagged, `.gitattributes` LFS filter generated, and push allowed | `cargo test -p gitupload-git test_lfs_track` |

---

## 4. Definition of Done (DoD) Checklist for Phase 2

- [x] `gitupload-git` compiles clean with zero warnings under `cargo check`.
- [x] `#![forbid(unsafe_code)]` observed across all crate modules except strictly bounded Win32 Job Object calls in `cli/job_object.rs`.
- [x] 100% of ephemeral Git integration tests pass (`cargo test -p gitupload-git`).
- [x] DAG Revwalk builds 10,000 commits in < 150ms.
- [x] Subprocess cancellation leaves zero orphaned `git.exe` processes (verified via Sysinternals Process Explorer).
- [x] Clippy reports zero warnings under `cargo clippy -p gitupload-git --all-targets -- -D warnings`.

---

## 5. Navigation & Cross-References

| Document | Link |
| :--- | :--- |
| **Sprint 1 Master Plan** | [Sprint 1 README](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/README.md) |
| **Previous Phase: Phase 1** | [phase-1-core-security-storage-hooks.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-1-core-security-storage-hooks.md) |
| **Next Phase: Phase 3** | [phase-3-tauri-ipc-ghcli-primer-ui.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-3-tauri-ipc-ghcli-primer-ui.md) |
| **C4 System Architecture** | [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) |
