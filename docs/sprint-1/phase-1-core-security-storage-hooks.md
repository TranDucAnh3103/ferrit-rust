# Sprint 1 — Phase 1: Core Domain, Security Vault, SQLite Storage & Native Git Hooks Specification
**Document Identifier:** SPRINT1-PHASE1-CORE-SECURITY-STORAGE-HOOKS  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Foundational Data Models, Domain Error Tree, Windows Keyring Integration, SQLite WAL Persistence Engine, Legacy Config Auto-Migrator, and Zero-Dependency Native Git Hooks  
**Target Crates:** `gitupload-core`, `gitupload-security`, `gitupload-storage`, `gitupload-hooks`  
**Timeline:** Days 1–5 (Sprint 1 Architecture Baseline & Security Core)  

---

## 1. Phase Overview & Objectives

Phase 1 establishes the rock-solid foundational infrastructure of the GitUpload Desktop application. It delivers the core types, thread-safe memory hygiene, encrypted OS credentials, relational local storage, and the zero-dependency pure-Rust Git hooks subsystem.

### Core Objectives:
1. **Centralized Domain Models & Strongly Typed Error Hierarchy (`gitupload-core`):** Define Serde-compatible data contracts for all system operations, comprehensive `thiserror` domain error variants, and deterministic naming slugification/validation.
2. **Zero-Plaintext Credential Vault & Log Sanitization (`gitupload-security`):** Connect directly to Windows Credential Manager via `keyring`, enforce in-memory zeroization via `secrecy::SecretString` and `zeroize`, and implement zero-allocation regex log scrubbing to prevent secret leakage.
3. **High-Performance SQLite Storage & Automatic Legacy Migration (`gitupload-storage`):** Initialize SQLite with Write-Ahead Logging (WAL) and synchronous normal mode. Create schema DDL for settings, push history, and organizations. Implement one-click automatic migration of legacy Go/Wails `%APPDATA%/GGitUpload/config.json` and `.env` files.
4. **Native Pure-Rust Git Hooks Engine (`gitupload-hooks`):** Implement a blazing-fast (<2ms), zero-dependency Git hooks runner in pure Rust replacing Node.js/Husky. Enforce **Conventional Commits 1.0.0**, scan staged blobs for leaked secrets, block files >50MB, and guard protected branches (`main`/`master`).

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 1 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |         TASK-P1-01          |  |          TASK-P1-02           |  |       TASK-P1-03        |  |
|  |     Domain Data Models      |  |   Centralized Error Tree      |  | Naming & Regex Parsers  |  |
|  |    (Push, DAG, Org, PR)     |  |    (thiserror Primitives)     |  | (Slugify, Student ID)   |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |      gitupload-core Crates   |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |         TASK-P1-04          |   |          TASK-P1-05          |   |       TASK-P1-06       |  |
|  |  Windows Keyring & Vault    |   |  SQLite WAL & Auto-Migrator  |   | Native Git Hook Runner |  |
|  |  + Token Log Sanitizer      |   |  (Settings, History, DDL)    |   | (Conventional Commits) |  |
|  |  (gitupload-security)       |   |  (gitupload-storage)         |   | (gitupload-hooks)      |  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-P1-01: Centralized Domain Models & Serde Schema Contracts

- **Task Identifier:** `TASK-P1-01`
- **Component:** `crates/gitupload-core/src/models/`
- **SRS Traceability:** `FR-GIT-001..016`, `FR-GH-001..012`, `FR-GRF-001`, `FR-ACC-001`

#### Implementation Specification
Implement all core transfer types with explicit Serde field renaming and validations:
1. `push.rs`: `PushPayload`, `PushMode` (`SESSION_EX`, `MINI_PROJECT`, `CUSTOM_REPOSITORY`), `PushResult`.
2. `graph.rs`: `GitCommitNode`, `BranchSwimlane`, `CommitRef`, `GraphEdge`.
3. `org.rs`: `GitHubOrg`, `OrgMembershipRole`.
4. `pr.rs`: `PullRequestInfo`, `PrState`, `MergeStrategy` (`Squash`, `Rebase`, `Merge`).
5. `settings.rs`: `AppSettings`, `NamingPattern`.

```rust
// crates/gitupload-core/src/models/push.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PushMode {
    SessionEx,
    MiniProject,
    CustomRepository,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPayload {
    pub folder_path: String,
    pub mode: PushMode,
    pub session_val: Option<String>,
    pub ex_val: Option<String>,
    pub mini_project_val: Option<String>,
    pub custom_repo_name: Option<String>,
    pub use_default_commit: bool,
    pub custom_commit_msg: Option<String>,
    pub is_file: bool,
    pub target_org: Option<String>,
    pub safe_sync: bool,
}
```

---

### Task TASK-P1-02: Domain Error Tree & Error Mapping

- **Task Identifier:** `TASK-P1-02`
- **Component:** `crates/gitupload-core/src/errors.rs`
- **SRS Traceability:** `NFR-REL-001`, `NFR-REL-002`, `NFR-MNT-001`

#### Implementation Specification
Establish a unified `thiserror::Error` enum hierarchy covering all domain failures. Every crate within the workspace must map internal errors into this strongly typed enum:

```rust
// crates/gitupload-core/src/errors.rs
use thiserror::Error;

pub type GitUploadResult<T> = Result<T, GitUploadError>;

#[derive(Error, Debug)]
pub enum GitUploadError {
    #[error("Filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization / Deserialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("OS Credential Vault error: {0}")]
    Vault(String),

    #[error("Git Engine error: {0}")]
    Git(String),

    #[error("GitHub API error [{status}]: {message}")]
    GitHubApi { status: u16, message: String },

    #[error("Native Hook validation failed: {0}")]
    Hook(String),

    #[error("GitHub CLI sandbox error: {0}")]
    GhCli(String),

    #[error("Cloud AI service error: {0}")]
    CloudAi(String),

    #[error("Storage / SQLite error: {0}")]
    Storage(String),

    #[error("Pre-flight safety guard blocked push: {0}")]
    SafetyGuard(String),

    #[error("Operation was cancelled by user")]
    Cancelled,
}
```

---

### Task TASK-P1-03: Repository Naming, Slugification & File Regex Parser

- **Task Identifier:** `TASK-P1-03`
- **Component:** `crates/gitupload-core/src/naming.rs`
- **SRS Traceability:** `FR-GIT-003`, `FR-GIT-004`, `FR-GIT-005`

#### Implementation Specification
Replicate and optimize 100% of legacy naming algorithms using pre-compiled `OnceLock<Regex>`:
1. `slugify_repo_name`: Converts arbitrary strings into GitHub-compliant repository names (strip invalid characters, collapse consecutive dashes, trim trailing dots/dashes).
2. `parse_file_metadata`: Extracts session number, exercise number, student name, and student ID from file patterns like `SS15_HW02_NguyenVanA_HE170001.java`.
3. `format_repo_name`: Replaces pattern tokens (`%02d`, `{session}`, `{exercise}`).

```rust
// crates/gitupload-core/src/naming.rs
use regex::Regex;
use std::sync::OnceLock;

static RE_SLUG_INVALID: OnceLock<Regex> = OnceLock::new();
static RE_DASHES: OnceLock<Regex> = OnceLock::new();
static RE_FILE_HOMEWORK: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHomeworkMetadata {
    pub session: u32,
    pub exercise: u32,
    pub student_name: Option<String>,
    pub student_id: Option<String>,
}

pub fn slugify_repo_name(name: &str) -> String {
    let re_invalid = RE_SLUG_INVALID.get_or_init(|| Regex::new(r"[^a-zA-Z0-9._-]").unwrap());
    let re_dashes = RE_DASHES.get_or_init(|| Regex::new(r"-+").unwrap());

    let trimmed = name.trim().replace(' ', "-");
    let cleaned = re_invalid.replace_all(&trimmed, "");
    let normalized = re_dashes.replace_all(&cleaned, "-");
    normalized.trim_matches(|c| c == '.' || c == '-').to_string()
}

pub fn parse_file_metadata(filename: &str) -> Option<FileHomeworkMetadata> {
    let re = RE_FILE_HOMEWORK.get_or_init(|| {
        Regex::new(r"(?i)SS(\d+)[_-](?:HW|EX)(\d+)(?:[_-]([a-zA-Z0-9]+))?(?:[_-]([a-zA-Z0-9]+))?").unwrap()
    });

    let caps = re.captures(filename)?;
    let session = caps.get(1)?.as_str().parse().ok()?;
    let exercise = caps.get(2)?.as_str().parse().ok()?;
    let student_name = caps.get(3).map(|m| m.as_str().to_string());
    let student_id = caps.get(4).map(|m| m.as_str().to_string());

    Some(FileHomeworkMetadata {
        session,
        exercise,
        student_name,
        student_id,
    })
}
```

---

### Task TASK-P1-04: Windows Credential Vault & Real-Time Token Log Sanitizer

- **Task Identifier:** `TASK-P1-04`
- **Component:** `crates/gitupload-security/src/keyring.rs`, `sanitizer.rs`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-002`, `NFR-SEC-003`

#### Implementation Specification
1. **OS Credential Manager Integration:**
   - Store credentials in Windows Credential Manager under target `GitUploadDesktop/<account>`.
   - Never write credentials to disk or SQLite columns.
   - Wrap retrieved tokens in `secrecy::SecretString` with `zeroize::Zeroize` on drop.
2. **Regex Log Sanitizer:**
   - Pre-compile patterns matching GitHub PATs (`ghp_*`, `github_pat_*`), Cloud AI keys (`sk-*`), and embedded HTTPS remote URLs (`https://<token>@github.com`).
   - Redact all tokens in log messages before disk writing or Tauri IPC emission.

```rust
// crates/gitupload-security/src/sanitizer.rs
use regex::Regex;
use std::sync::OnceLock;

static RE_URL_TOKEN: OnceLock<Regex> = OnceLock::new();
static RE_GH_PAT: OnceLock<Regex> = OnceLock::new();

pub fn mask_sensitive_tokens(raw: &str) -> String {
    let re_url = RE_URL_TOKEN.get_or_init(|| {
        Regex::new(r"https://[^@\s]+@github\.com").unwrap()
    });
    let re_pat = RE_GH_PAT.get_or_init(|| {
        Regex::new(r"(ghp_[a-zA-Z0-9]{36}|github_pat_[a-zA-Z0-9_]{82}|sk-[a-zA-Z0-9]{20,})").unwrap()
    });

    let masked_urls = re_url.replace_all(raw, "https://***@github.com");
    re_pat.replace_all(&masked_urls, "***").to_string()
}
```

---

### Task TASK-P1-05: SQLite Persistence Pool, Schema DDL & Legacy Data Auto-Migrator

- **Task Identifier:** `TASK-P1-05`
- **Component:** `crates/gitupload-storage/src/db.rs`, `schema.rs`, `migrator.rs`
- **SRS Traceability:** `NFR-PERF-003`, `FR-SET-001`, `FR-GH-001`

#### Implementation Specification
1. **Connection Pooling & WAL Mode:**
   - Use `r2d2_sqlite` with connection limit 8.
   - Execute `PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;`.
2. **Schema DDL:**
   - `app_settings` (key, value)
   - `push_history` (id, repo_name, repo_url, local_path, commit_msg, pushed_at)
   - `organizations` (login, id, avatar_url, is_admin)
   - `pull_requests` (id, number, repo_full_name, title, state, html_url, created_at)
   - `hooks_config` (key, value)
3. **Legacy Auto-Migrator (`migrator.rs`):**
   - Check for legacy `%APPDATA%/GGitUpload/config.json`, `%APPDATA%/GGitUpload/history.json`, and local `.env`.
   - Extract `github_token` and `ai_api_key` directly into Windows Credential Manager via `KeyringManager`.
   - Migrate settings into SQLite `app_settings` table and push history into `push_history`.
   - Rename legacy files to `.migrated_backup` and wipe plaintext credentials.

---

### Task TASK-P1-06: Zero-Dependency Native Git Hooks Engine

- **Task Identifier:** `TASK-P1-06`
- **Component:** `crates/gitupload-hooks/src/runner.rs`, `config.rs`
- **SRS Traceability:** `FR-HOK-001`, `FR-HOK-002`, `FR-HOK-003`, `FR-HOK-004`, `FR-HOK-005`

#### Implementation Specification
Implement a pure-Rust hook runner executing under 2ms:
1. `commit-msg`: Validates message against Conventional Commits 1.0.0 (`feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `build:`, `ci:`, `chore:`, `revert:`). Enforces maximum header length of 72 characters.
2. `pre-commit`:
   - Scans staged files for sensitive tokens (`ghp_*`, `aws_key`, private keys).
   - Flags files exceeding size limit (default > 50MB, advises Git LFS).
   - Rejects `.env` files from staging.
3. `pre-push`:
   - Checks target branch against `prevent_direct_push_branches` (`main`, `master`).
   - If protected, blocks direct push and prompts user to create a Pull Request.
4. Auto-generates `.gitupload/hooks.toml` in repository root.

```rust
// crates/gitupload-hooks/src/runner.rs
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use regex::Regex;
use std::sync::OnceLock;

static RE_CONVENTIONAL: OnceLock<Regex> = OnceLock::new();

pub struct HookRunner;

impl HookRunner {
    pub fn validate_commit_msg(msg: &str, max_len: usize) -> GitUploadResult<()> {
        let first_line = msg.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            return Err(GitUploadError::Hook("Commit message cannot be empty".into()));
        }
        if first_line.len() > max_len {
            return Err(GitUploadError::Hook(format!(
                "Commit header exceeds maximum allowed length of {} chars (actual: {})",
                max_len,
                first_line.len()
            )));
        }

        let re = RE_CONVENTIONAL.get_or_init(|| {
            Regex::new(r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-zA-Z0-9._-]+\))?!?: .+").unwrap()
        });

        if !re.is_match(first_line) {
            return Err(GitUploadError::Hook(format!(
                "Commit message does not adhere to Conventional Commits standard: '{}'",
                first_line
            )));
        }

        Ok(())
    }
}
```

---

## 3. Strict Acceptance Criteria (AC-xxx)

### Acceptance Criteria Table

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-P1-01** | An unformatted repo name `"  Session 15 - Ex 02  "` | Calling `slugify_repo_name()` | Returns `"Session-15-Ex-02"` with zero consecutive dashes or illegal characters | `cargo test -p gitupload-core test_slugify` |
| **AC-P1-02** | A filename `"SS15_HW02_DucAnh_HE170001.java"` | Calling `parse_file_metadata()` | Returns `FileHomeworkMetadata` with `session=15`, `exercise=2`, `student_id="HE170001"` | `cargo test -p gitupload-core test_file_meta` |
| **AC-P1-03** | A sensitive GitHub PAT `"ghp_1234567890abcdef1234567890abcdef12"` | Storing via `KeyringManager::store_token()` | Token is encrypted in Windows Credential Manager and not present in SQLite or filesystem | `cargo test -p gitupload-security test_keyring` |
| **AC-P1-04** | A log line containing `"git remote add origin https://ghp_abc@github.com/org/repo"` | Passing through `mask_sensitive_tokens()` | Output is redacted to `"https://***@github.com/org/repo"` with zero token leakage | `cargo test -p gitupload-security test_masking` |
| **AC-P1-05** | An initialized `StoragePool` | Executing query and writing history | SQLite operates in WAL mode; write-and-read finishes in < 2ms | `cargo test -p gitupload-storage test_wal` |
| **AC-P1-06** | Legacy `config.json` containing plaintext tokens | Running `LegacyImporter::migrate_if_needed()` | Tokens are moved to Windows Keyring, settings saved to SQLite, and file renamed to `.migrated_backup` | `cargo test -p gitupload-storage test_migration` |
| **AC-P1-07** | A commit message `"feat(auth): add keyring support"` | Validating via `HookRunner::validate_commit_msg()` | Returns `Ok(())` | `cargo test -p gitupload-hooks test_conventional` |
| **AC-P1-08** | A non-conventional commit message `"fixed bugs"` | Validating via `HookRunner::validate_commit_msg()` | Returns `Err(GitUploadError::Hook(...))` with actionable remediation guidance | `cargo test -p gitupload-hooks test_invalid_msg` |

---

## 4. Definition of Done (DoD) Checklist for Phase 1

- [x] All 4 crates (`gitupload-core`, `gitupload-security`, `gitupload-storage`, `gitupload-hooks`) compile clean under `cargo check`.
- [x] `#![forbid(unsafe_code)]` enforced across all Phase 1 crate roots.
- [x] 100% of unit tests pass with zero failures (`cargo test -p gitupload-core -p gitupload-security -p gitupload-storage -p gitupload-hooks`).
- [x] Zero plaintext credentials stored on disk; verified via filesystem inspection of test outputs.
- [x] Clippy reports zero warnings under `cargo clippy -p gitupload-core -p gitupload-security -p gitupload-storage -p gitupload-hooks --all-targets -- -D warnings`.

---

## 5. Navigation & Cross-References

| Document | Link |
| :--- | :--- |
| **Sprint 1 Master Plan** | [Sprint 1 README](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/README.md) |
| **Next Phase: Phase 2** | [phase-2-git-engine-graph-safesync.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-2-git-engine-graph-safesync.md) |
| **Software Requirements Specification** | [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md) |
| **C4 System Architecture** | [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) |
