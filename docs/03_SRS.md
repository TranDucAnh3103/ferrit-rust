# Software Requirements Specification (SRS)
## GitUpload Desktop - Modern High-Performance Git Automation & Productivity Suite
### Standard: IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant

---

**Document Identifier:** SRS-RUST-GITUPLOAD-V2.2  
**Initial Baseline Date:** November 18, 2026  
**Revision Status:** Approved for Implementation (v2.2 Expansion)  
**Target Platform:** Windows 10/11 (Primary, x86_64/ARM64), Linux (x86_64/ARM64), macOS (Apple Silicon/Intel)  
**Implementation Technology:** Rust (Tokio, libgit2 / git2-rs, Keyring) + Tauri v2 + Vanilla/Tailwind Web Frontend  
**Predecessor System:** G-GitUpload v1.4.1 (Go/Wails v2 Desktop Tool)  

> **Associated Technical Documentation Suite:**
> - [01_LEGACY_AUDIT.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/01_LEGACY_AUDIT.md) - Legacy Go/Wails Audit & Vulnerability Assessment
> - [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) - Target Rust Architecture & C4 Models
> - [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md) - Phased Implementation & Migration Roadmap
> - [README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/README.md) - Project Overview & Quickstart

---

## Table of Contents

1. [Introduction](#1-introduction)
   - 1.1 Purpose
   - 1.2 Document Conventions & Formatting
   - 1.3 Intended Audience & Reading Suggestions
   - 1.4 Product Scope & Objectives
   - 1.5 Standards Compliance & References
2. [Overall Description](#2-overall-description)
   - 2.1 Product Perspective & Context
   - 2.2 System Operational Flow & Architecture Topology
   - 2.3 User Classes & Personas
   - 2.4 Operating Environment & Target Specifications
   - 2.5 Design & Implementation Constraints
   - 2.6 Assumptions & Dependencies
3. [Specific Functional Requirements (FR-xxx)](#3-specific-functional-requirements)
   - 3.1 Git Core Operations & Local Repository Management (FR-GIT)
   - 3.2 GitHub Integration, Organizations & Telemetry (FR-GH)
   - 3.3 Multi-Repo Batch Operations (FR-BAT)
   - 3.4 Smart Conflict Resolution Visualizer (FR-CRV)
   - 3.5 Cloud AI Copilot & Multi-Provider Engine (FR-AI)
   - 3.6 Multi-Account & Multi-Profile Management (FR-ACC)
   - 3.7 Background Git Watcher & Auto-Sync Engine (FR-WAT)
   - 3.8 Settings, Customization & System Utilities (FR-SET)
   - 3.9 Pull Request (PR) Lifecycle Management (FR-PR)
   - 3.10 Native Git Hooks Engine (FR-HOK)
   - 3.11 Interactive Git Graph & Branch Tree Visualizer (FR-GRF)
   - 3.12 GitHub CLI (`gh`) Full Lifecycle Manager & Auth Bridge (FR-GHCLI)
   - 3.13 GitHub Actions, Issues, Releases & Developer Automations (FR-ACT, FR-ISS, FR-REL, FR-AUT)
4. [External Interface Requirements](#4-external-interface-requirements)
   - 4.1 User Interfaces - GitHub Primer Design System Fidelity (UI/UX)
   - 4.2 Hardware & System Interfaces
   - 4.3 Software & Native Operating System Interfaces
   - 4.4 Communications & Network Protocol Interfaces
5. [Rigorous Non-Functional Requirements (NFR-xxx)](#5-rigorous-non-functional-requirements)
   - 5.1 Performance Requirements (NFR-PERF)
   - 5.2 Security & Credential Vault Requirements (NFR-SEC)
   - 5.3 Reliability, Fault Tolerance & Recovery (NFR-REL)
   - 5.4 Portability & Cross-Platform Compliance (NFR-PORT)
   - 5.5 Usability, Internationalization & Accessibility (NFR-USE)
   - 5.6 Maintainability, Modularity & Testability (NFR-MNT)
6. [Legacy Codebase Feature Traceability Matrix](#6-legacy-codebase-feature-traceability-matrix)
7. [Verification, Quality Assurance & Acceptance Criteria](#7-verification-quality-assurance--acceptance-criteria)

---

## 1. Introduction

### 1.1 Purpose
This Software Requirements Specification (SRS) establishes the formal, binding functional and non-functional specifications for the rebuilt **GitUpload Desktop Suite** (codenamed `rust-gitupload`). 

The system completely modernizes and supersedes the legacy `G-GitUpload v1.4.1` application (developed with Go and Wails v2). This SRS guarantees **100% backward feature parity** with the legacy system while introducing next-generation capabilities:
- Native multi-threaded Git operations using compiled C-bindings (`libgit2` via `git2-rs`) backed by an asynchronous Tokio actor model.
- High-throughput multi-repo batch synchronization with real-time aggregated streaming progress.
- Native Operating System Credential Storage eliminating all plaintext tokens on disk.
- Pure Cloud AI acceleration (Google Gemini 2.0/1.5, OpenAI GPT-4o, Claude 3.5 Sonnet, OpenRouter, Groq) with intelligent commit generation, automated error diagnosis, PR reviews, and diff explanation.
- Native Git Hooks Engine (Husky/Lefthook equivalent in pure Rust) providing zero-dependency commit linting and pre-commit/pre-push validations without Node.js or npm.
- GitHub Organizations support with instant context switching, permission verification, and direct org repository creation.
- 1:1 GitHub-Fidelity UI/UX styled with the GitHub Primer Design System, Primer Octicons, and authentic file tree and diff viewers.
- Re-push and synchronization from existing local repositories with remote tracking inspection and safe push/force-with-lease options.
- Complete Pull Request (PR) lifecycle management: creation, draft toggle, diff inspection, reviews, and multi-strategy merging.
- Interactive Git Graph & Branch Tree Visualizer (DAG commit graph, branch swimlanes, interactive rebase, cherry-pick, reset, and commit inspector).
- GitHub CLI (`gh`) Full Lifecycle Manager: automatic detection, one-click silent installer into user sandbox, auto-updater, and dual-auth credential bridge (extracts token via `gh auth token` into Windows Keyring with zero manual PAT copying).
- GitHub Developer Automation Suite: GitHub Actions workflow monitor with ANSI log streaming, in-app Issue tracker, Release manager with AI release notes, Gist manager, fork & upstream synchronization, and auto-stash/branching automations.
- Built-in visual 3-way merge conflict detection and resolution.
- Background filesystem watcher with debounced auto-commit and sync pipelines.

### 1.2 Document Conventions & Formatting
- **Requirement Identifiers:** 
  - `FR-<CATEGORY>-<NUM>` denotes a **Functional Requirement**.
  - `NFR-<CATEGORY>-<NUM>` denotes a **Non-Functional Requirement**.
- **Requirement Priorities:** Defined in accordance with RFC 2119:
  - **SHALL / MUST [P1]:** Critical core requirement. Mandatory for MVP release.
  - **SHOULD [P2]:** High-priority feature. Essential for enhanced workflow and stability.
  - **MAY [P3]:** Desirable enhancement for subsequent minor revisions.
- **Traceability References:** Cross-references to legacy source files in `F:\Build\JavaToolGitAutoUpload\g-gitupload` are explicitly cited.

### 1.3 Intended Audience & Reading Suggestions
- **Software Architects & Engineers (@architect):** Guide implementation of Tokio task loops, actor mailboxes, IPC schema bindings, and libgit2 error mappings.
- **Project Lead & Roadmap Authors (@roadmap_author):** Plan work breakdown schedules, dependency topologies, and milestone milestones.
- **Quality Assurance & Swarm Verifiers (@verifier):** Derive formal acceptance test matrices, benchmark stress tests, and security audit suites.

### 1.4 Product Scope & Objectives
GitUpload Desktop is an ultra-lightweight, cross-platform graphical productivity client for Git and GitHub. It serves students, software engineers, and automated submission workflows.
The objective is to replace a monolithic, high-overhead runtime (~150MB RAM, plaintext `.env` storage, unbuffered command execution, forced pushes) with a native, memory-safe, ultra-responsive Rust system:
- Memory footprint: **< 30 MB idle RAM** (compared to 150MB+ in Wails WebView2).
- Cold startup latency: **< 100 ms** to interactive UI.
- Credential Security: **100% hardware-backed or OS-level encrypted vault** (Windows Credential Manager / macOS Keychain / Linux Secret Service).
- Data Safety: Zero destructive forced pushes without explicit user confirmation, pre-flight conflict inspection, and automatic stashing.

### 1.5 Standards Compliance & References
1. **IEEE Std 830-1998:** Recommended Practice for Software Requirements Specifications.
2. **ISO/IEC/IEEE 29148:2018:** Systems and software engineering — Life cycle processes — Requirements engineering.
3. **RFC 2119:** Key words for use in RFCs to Indicate Requirement Levels.
4. **Git Documentation:** The Git SCM Protocol and Plumbings (git-scm.com).
5. **GitHub REST API v3 / GraphQL v4 Specification:** (docs.github.com/en/rest).

---

## 2. Overall Description

### 2.1 Product Perspective & Context
The predecessor `G-GitUpload` solved manual friction in submitting programming assignments and creating remote repositories. However, its architectural model suffered from several core vulnerabilities and bottlenecks:
1. **Security Vulnerability:** Sensitive GitHub Personal Access Tokens (PAT) and AI API keys were written in plaintext to `%APPDATA%\GGitUpload\config.json` and `.env` files.
2. **Subprocess Churn:** Every Git command (`git init`, `git add`, `git commit`, `git push`) spawned a raw Windows `cmd.exe`/`git.exe` subprocess via `os/exec.Command`, risking Windows handle exhaustion and UI stutters.
3. **Data Loss Hazard:** The legacy push routine executed `git push -u origin main --force --progress` unconditionally, risking accidental overwrites of existing remote repositories.
4. **Single-Target Limitation:** Only one repository or file could be processed at a time.
5. **Tightly-Coupled Cloud AI:** AI features relied entirely on external OpenRouter endpoints, failing completely when offline or rate-limited.

```
+--------------------------------------------------------------------------------------------------+
|                                  GitUpload Desktop Suite                                         |
|                                                                                                  |
|  +--------------------------------------------------------------------------------------------+  |
|  |                 GitHub-Fidelity Primer Web Frontend (HTML5 / Tailwind / Primer CSS)        |  |
|  |  [Push/Re-Push]  [Batch]  [Clone]  [Search]  [Conflict Visualizer]  [PR Hub]  [Hooks]  [AI]  |  |
|  +----------------------------------------------+---------------------------------------------+  |
|                                                 | Tauri IPC (JSON-RPC 2.0 / Streams)             |
|  +----------------------------------------------v---------------------------------------------+  |
|  |                               Rust Core Engine (Tauri v2 Core)                             |  |
|  |  +--------------------+  +--------------------+  +-------------------+  +---------------+  |  |
|  |  | Git Actor Engine   |  | Batch Orchestrator |  | Git Hooks Engine  |  | Cloud AI Hub  |  |  |
|  |  | (Tokio / libgit2)  |  | (Tokio Semaphore)  |  | (Rust-native)     |  | (Multi-Cloud) |  |  |
|  |  +---------+----------+  +---------+----------+  +---------+---------+  +-------+-------+  |  |
|  |  +---------v-----------------------v-----------------------v-------------------v----------+  |
|  |  |                  GitHub Service (REST v3 / GraphQL v4 / PR Lifecycle)                  |  |
|  |  +-----------------------------------------+----------------------------------------------+  |
|  +--------------------------------------------|-----------------------------------------------+  |
|                                               |                                                  |
|  +--------------------------------------------v-----------------------------------------------+  |
|  |                             Operating System & Platform Services                           |  |
|  |  +-----------------------------+  +----------------------------+  +---------------------+  |  |
|  |  | OS Credential Manager       |  | Relational SQLite (WAL)    |  | Cloud AI Services   |  |  |
|  |  | (Windows Keyring / WinCred) |  | (History, Profiles, Cache) |  | (Gemini, OpenAI,    |  |  |
|  |  |                             |  |                            |  | Claude, Groq)       |  |  |
|  |  +-----------------------------+  +----------------------------+  +---------------------+  |  |
|  +--------------------------------------------------------------------------------------------+  |
+--------------------------------------------------------------------------------------------------+
```

### 2.2 System Operational Flow & Architecture Topology
The rewritten Rust architecture separates concerns across modular boundaries:
- **Presentation Layer (Frontend):** Authentic GitHub-fidelity interface built with GitHub Primer Design System tokens, Octicons, CSS Flexbox/Grid, and zero-overhead native bindings. Communicates with Rust through Tauri v2 type-safe commands and event channels.
- **Asynchronous Task Bus (Tokio Actor Model):** Long-running Git operations (cloning, packing objects, diffing, batch processing, pushing) are queued into dedicated Tokio worker actors. The GUI never blocks and consistently renders at 60+ FPS.
- **Native Git Engine (`git2-rs` with CLI Fallback):** Direct in-process C-level access to Git repositories via `libgit2`. If complex custom SSH agent configs or submodules require CLI operations, the engine seamlessly invokes an asynchronous fallback runner wrapped in Windows Job Objects.
- **Native Git Hooks Engine:** Built-in zero-dependency hook orchestrator replacing Husky/Lefthook. Manages `.git/hooks` scripts that call back into the native Rust binary for sub-millisecond commit linting and pre-commit checks without Node.js or npm.
- **Secure Vault Layer (`keyring-rs`):** Interacts directly with Windows Credential Manager (`wincred`), macOS Keychain, or Linux Secret Service (`libsecret`). Zero plaintext secrets exist on filesystem storage.
- **Pure Cloud AI Service:** Ultra-responsive multi-cloud LLM client supporting Google Gemini 2.0/1.5, OpenAI GPT-4o, Anthropic Claude 3.5 Sonnet, Groq Llama-3.3-70B, and OpenRouter, eliminating heavy local LLM dependencies.

### 2.3 User Classes & Personas
- **Persona 1: Student / Learner ("Bao"):** Submits homework exercises and single Markdown reports rapidly. Needs drag-and-drop convenience, auto-generated repository naming patterns, auto-commit formatting, and quick IDE launching without understanding Git command line internals.
- **Persona 2: Teaching Assistant / Grader ("Stephen"):** Performs batch clone/pull of student submissions, reviews code diffs, searches repositories across organizations, and verifies submissions.
- **Persona 3: Professional Developer ("Mai"):** Manages multiple Git profiles (Work vs. Personal), works within GitHub Organizations, requires Pull Request lifecycle reviews and branch protection compliance, and relies on Cloud AI assistants for conventional commits and PR descriptions.

### 2.4 Operating Environment & Target Specifications
- **Operating Systems:**
  - Windows 10 (Build 19041+) and Windows 11 (64-bit and ARM64) - Tier 1 Primary Target.
  - Linux distributions: Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch Linux (x86_64, aarch64).
  - macOS: 12.0 Monterey or higher (Apple Silicon M-series & Intel x86_64).
- **Runtime Environment:** Native standalone binary with embedded web assets (zero external runtime dependencies such as Node.js or Go runtime required).
- **Display Resolution:** Optimized for minimum 1024x600 up to 4K UHD with native DPI scaling.

### 2.5 Design & Implementation Constraints
- **CON-01:** Implementation language for backend logic MUST be 100% Rust (2021 edition or later).
- **CON-02:** Desktop windowing and IPC MUST utilize Tauri v2.
- **CON-03:** Secret credentials (GitHub PATs, OAuth tokens, Cloud AI API keys) MUST NEVER be stored in plaintext JSON, `.env`, or configuration files on the filesystem.
- **CON-04:** Subprocesses MUST not produce flashing black console windows on Windows (`CREATE_NO_WINDOW = 0x08000000`).
- **CON-05:** Application MUST run in single-instance mode enforced by native OS mutex primitives.
- **CON-06:** AI engine MUST be strictly Cloud AI based (Gemini, OpenAI, Claude, Groq, OpenRouter); heavy local LLM runtimes (Ollama/llama-cpp) are prohibited to maintain a lightweight system footprint.

### 2.6 Assumptions & Dependencies
- Users utilizing GitHub features possess active GitHub accounts and network connectivity.
- Cloud AI features require an active internet connection and configured API key (stored securely in the OS Keyring) or shared organization key.
- Git CLI executable is optional for 95% of operations handled directly by `libgit2`, but can be utilized as a secondary fallback if installed on `PATH`.

---

## 3. Specific Functional Requirements

### 3.1 Git Core Operations (FR-GIT)

#### FR-GIT-001: Git Repository Initialization
- **Description:** The system SHALL initialize a new Git repository within the selected project directory if a `.git` folder does not already exist.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `git/runner.go:PushHomework` (`git init`).
- **Input:** Target directory path string.
- **Processing:** Invokes `git2::Repository::init(path)`. Configures repository settings (`gc.auto = 0`). Ensures standard `.gitignore` template is created if absent.
- **Output:** Initialized repository handle and log notification.

#### FR-GIT-002: Dynamic Subdirectory & Single File Isolation Mode
- **Description:** The system SHALL support two distinct project upload modes:
  1. **Folder Mode:** Stages the entire directory tree.
  2. **Single File Mode:** Isolates a single selected file (e.g., `.md`, `.java`, `.rs`, `.py`), copies it to an isolated sandboxed workspace, and packages it into a dedicated repository without dirtying the parent directory.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `push_service.go:StartPush` (lines 86-106).
- **Input:** File path, upload mode flag (`Folder` | `SingleFile`).
- **Processing:** If single file, the system creates a secure temp folder via `tempfile::Builder`, copies the target file, and initiates Git staging inside the isolated folder.

#### FR-GIT-003: Intelligent Single-File Filename Parser
- **Description:** In Single File mode, the system SHALL parse structured filenames matching assignment conventions to automatically populate session, exercise, student name, and student ID fields.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `push_service.go:resolveCommitMsgForFile` (lines 351-365).
- **Regular Expression Rules:**
  - Session/Exercise match: `(?i)SS(\d+)[_-](?:HW|EX)(\d+)`
  - Name/Student ID tokenization: Split normalized `_` segments `[Prefix, Exercise, StudentName, StudentID]`.
- **Output:** Populated form state (`session`, `exercise`, `student_name`, `student_id`).

#### FR-GIT-004: Flexible Repository Naming Pattern Engine
- **Description:** The system SHALL format the remote repository name based on user-selected strategies:
  - `SESSION_EX`: Formatted via configurable pattern (e.g. `session%s-ex%s-homework` or `session{session}-ex{exercise}`).
  - `MINI_PROJECT`: Formatted as `session{session:02}-miniproject-{slug}`.
  - `CUSTOM_REPOSITORY`: User-provided alphanumeric repository name.
  - `SYNC_WITH_COMMIT`: Automatically slugifies the generated commit message into a GitHub-compliant repository name.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `push_service.go:resolveRepoName`, `slugifyRepoName`.
- **Validation:** Must be 1 to 100 characters; only `[a-zA-Z0-9._-]`; cannot start or end with `.` or `-`; cannot contain `..`.

#### FR-GIT-005: Safe Staging & Smart .gitignore Sanitization
- **Description:** Prior to staging files, the system SHALL inspect `.gitignore` and enforce that sensitive patterns (`.env`, `*.key`, `*.pem`, `id_rsa`, `secrets.json`, `.DS_Store`, `Thumbs.db`) are ignored.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `git/runner.go:ensureEnvIgnored`.
- **Processing:** Appends missing safety patterns to `.gitignore` automatically with user audit notice. Executes staging via `git2::Index::add_all`.

#### FR-GIT-006: Identity-Aware Commit Engine
- **Description:** The system SHALL generate a signed or standard Git commit with configurable commit messages and author identity.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `push_service.go:156-164`, `git/runner.go:135-144`.
- **Inputs:** Author Name, Author Email (falling back to GitHub username and `username@users.noreply.github.com`), Commit Message, Empty commit allowance flag.
- **Processing:** Creates a Git commit tree using `libgit2` commit APIs with microsecond timestamps and timezone preservation.

#### FR-GIT-007: Safe Push Protocol with Remote Collision Check
- **Description:** The system SHALL push the local branch to the remote GitHub repository. Unlike the legacy system which forced pushes (`--force`), the modern system SHALL perform a pre-flight inspection of remote references.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `git/runner.go:183-186` (Replaced unsafe `push --force`).
- **Processing:**
  - If remote branch does not exist, performs initial push (`refs/heads/main`).
  - If remote branch exists and local is ahead, pushes cleanly.
  - If remote branch is ahead (diverged), blocks the push, notifies the user, and opens the **Smart Conflict Resolution Visualizer** (FR-CRV).
  - Force Push is permitted ONLY when the user explicitly enables an override checkbox in Advanced Settings.

#### FR-GIT-008: Real-Time Git Progress & Streaming Telemetry
- **Description:** The system SHALL stream real-time progress percentages and log messages during push and clone operations via asynchronous Tauri IPC events (`git_progress`, `git_log`).
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `git/runner.go:SplitByNewlineOrCR`, `push_service.go:167-174`.
- **Processing:** Implements `git2::PackBuilder` transfer progress callbacks and regex scanner for CLI fallbacks. Automatically masks tokens and secrets matching `https://.*@github.com` with `https://***@github.com`.

#### FR-GIT-009: Repository Cloning with Real-Time Delta Decoding
- **Description:** The system SHALL clone remote Git repositories to a designated local path with real-time transfer and checkout progress monitoring.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `clone_service.go:StartClone`.
- **Processing:** Sets credentials via Git credential callbacks; monitors transfer percentages (receiving objects, resolving deltas); checks out default branch cleanly; masks authorization tokens.

#### FR-GIT-010: Installed IDE Discovery & Quick Launcher
- **Description:** Following successful clone or push operations, the system SHALL scan the host system for installed programming IDEs and render one-click launch buttons.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `clone_service.go:DetectIDEs` (lines 105-228).
- **Supported IDEs:**
  - Visual Studio Code (`Code.exe`, `code` CLI)
  - IntelliJ IDEA (Community, Ultimate, JetBrains Toolbox apps)
  - Cursor (`Cursor.exe`, `cursor` CLI)
  - Antigravity IDE (`Antigravity.exe`, `antigravity` CLI)
  - System File Explorer (Windows Explorer, Finder, xdg-open)
- **Processing:** Launches target IDE process detached with `CreationFlags(0x08000000)` without window flickering.

#### FR-GIT-011: Git Branch Lifecycle Manager
- **Description:** The system SHALL list local and remote branches, allow creating new branches, switching active branches (checkout), and deleting merged branches.
- **Priority:** P2 (High)
- **Processing:** Interacts with `git2::Branch` and updates UI status bar with active branch badge.

#### FR-GIT-012: Working Tree Status & Stash Storage
- **Description:** The system SHALL inspect uncommitted working tree changes (modified, untracked, deleted, renamed files) and provide quick Stash / Pop capabilities to safeguard uncommitted work during branch switches or sync operations.
- **Priority:** P2 (High)

#### FR-GIT-013: Commit History Log & Interactive Graph
- **Description:** The system SHALL display commit history for the active repository, showing commit SHA, author, timestamp, message, and visual branch/merge lines.
- **Priority:** P2 (High)

#### FR-GIT-014: Zero-Copy Local File Diff Inspector
- **Description:** The system SHALL generate structured syntax-highlighted diffs between working directory files and `HEAD` using zero-copy memory buffers.
- **Priority:** P2 (High)

#### FR-GIT-015: Operation Cancellation & Resource Cleanup
- **Description:** The user SHALL have the ability to cancel an active push, clone, or batch operation at any time.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `push_service.go:CancelPush`.
- **Processing:** Signals Tokio `CancellationToken`. Aborts network sockets immediately, cleans temporary workspace folders, and resets progress bars.

#### FR-GIT-016: Re-Push & Synchronization from Existing Local Repositories
- **Description:** When a user selects or drops a folder already initialized with a `.git` repository, the system SHALL automatically detect the repository state rather than forcing re-initialization.
- **Priority:** P1 (Mandatory)
- **Processing:**
  - Inspects local branch name, remotes (`origin`), and upstream tracking branch.
  - Computes `ahead` and `behind` commit counters against the remote tracking branch.
  - Supports incremental staging (`git add`), conventional commit, and safe push.
  - If history rewrite or rebase is detected, provides safe `--force-with-lease` push option (guaranteeing remote is not overwritten if someone else pushed concurrently) rather than raw `--force`.
  - Displays real-time branch sync pill: `[Up-to-date]`, `[Ahead by X commits]`, `[Behind by Y commits]`, or `[Diverged]`.

#### FR-GIT-017: Git LFS (Large File Storage) Tracking & Object Management
- **Description:** The system SHALL detect binary and large assets exceeding configurable thresholds (>50MB or >100MB) and offer automated Git LFS tracking.
- **Priority:** P2 (High)
- **Processing:**
  - Automatically updates `.gitattributes` with `filter=lfs diff=lfs merge=lfs -text` for identified large asset patterns.
  - Ensures Git LFS pointers are created and large binary objects are uploaded to GitHub LFS storage endpoints via Git LFS API.
  - Prevents GitHub HTTP 413 / `File exceeds 100.00 MB` push rejection errors.

#### FR-GIT-018: Declarative .gitignore Generator
- **Description:** The system SHALL provide a built-in `.gitignore` generator with offline templates for 100+ programming languages, frameworks, and operating systems (e.g. Java, Maven, Gradle, Rust, Python, Node, React, Windows, macOS).
- **Priority:** P2 (High)
- **Processing:** Searchable multi-tag selector merges standard community rules, deduplicates patterns, and writes directly to project `.gitignore`.

#### FR-GIT-019: Cryptographic Commit Signing (GPG / SSH) & Verification
- **Description:** The system SHALL support cryptographic commit signing using either local GPG keys (`user.signingkey`) or modern SSH keys (`gpg.format = ssh`).
- **Priority:** P2 (High)
- **Processing:** Integrates with local signing agents; generates signature blocks on commit objects; displays GitHub-faithful `[Verified]` green badge on commit logs.

---

### 3.2 GitHub Integration & Management (FR-GH)

#### FR-GH-001: GitHub Personal Access Token (PAT) Authentication
- **Description:** The system SHALL authenticate with GitHub API v3 using Personal Access Tokens (Classic `ghp_...` and Fine-grained `github_pat_...`).
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `github/client.go:GetGitHubProfile`, `GetTokenReport`.
- **Processing:** Validates token against `https://api.github.com/user`. Encrypts and persists valid tokens directly into the native OS Keyring.

#### FR-GH-002: GitHub OAuth 2.0 Device Code Flow
- **Description:** In addition to manual PAT entry, the system SHALL support GitHub OAuth Device Code Flow (`https://github.com/login/device/code`).
- **Priority:** P2 (High - Major Modern Feature)
- **Processing:** The application requests a device code, displays an 8-character user verification code, copies it to clipboard, opens `https://github.com/login/device` in the browser, and polls the GitHub OAuth endpoint asynchronously until the user authorizes access. Stores the resulting OAuth access token in the OS Keyring. Eliminates manual PAT generation friction for beginners.

#### FR-GH-003: GitHub Organizations Management & Context Switching
- **Description:** The system SHALL provide full, first-class support for GitHub Organizations, allowing users to toggle between Personal accounts and any affiliated Organization.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `config/config.go:HasOrganization`, `github/client.go:CreateRepository`.
- **Processing:**
  - Queries `GET /user/orgs` to list all joined organizations, their avatar URLs, and descriptions.
  - Checks user membership role (`admin`, `member`) and effective repository permissions (`admin:org`, `write:org`).
  - Allows selecting an Organization as the active context: searches, filters, and displays organization-owned repositories.
  - Automatically routes new repository creation to `POST /orgs/{org}/repos` when an organization context is selected, verifying organization private repository creation policies.
  - Surfaces organization badge next to repository names in the UI.

#### FR-GH-004: Remote Repository Creation with Empty-Repo Collision Handling
- **Description:** The system SHALL automatically create the remote GitHub repository (Public or Private). If a repository with the identical name already exists, the system SHALL check if the remote repository is completely empty (`size == 0`).
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `github/client.go:CreateRepository` (lines 90-155).
- **Processing:** If empty, the system reuses the existing repository and continues the push. If non-empty, the system reports a collision conflict and suggests an incremented suffix (e.g., `-v2`).

#### FR-GH-005: Server-Side Repository Search & Pagination
- **Description:** The system SHALL search GitHub repositories with server-side query filtering, pagination (12 to 50 items/page), and sorting (`default` updated, `empty` first, `new` within 24h).
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `search_service.go:SearchRepos`, `github/client.go:SearchRepositories`.

#### FR-GH-006: Visual Status Badges ([EMPTY] and [NEW])
- **Description:** Repositories rendered in search results SHALL display real-time status badges:
  - `[EMPTY]`: Repository exists but contains 0 commits / 0 bytes.
  - `[NEW]`: Repository was updated or pushed within the past 24 hours.
  - Primary language breakdown tag with percentage indicator.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `github/client.go:SortRepositories`, `GetRepoLanguageStats`.

#### FR-GH-007: Remote Tree & File Explorer
- **Description:** The system SHALL allow users to browse remote repository directory trees and view file contents directly within the desktop UI without cloning.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `search_service.go:GetRepoContents`, `GetRepoFile`.

#### FR-GH-008: Direct In-App File Editor & Commit Push
- **Description:** The system SHALL allow users to edit text/markdown files (e.g. `README.md`) directly in the app and commit changes back to GitHub via GitHub API.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `search_service.go:UpdateRepoFile`, `github/client.go:UpdateRepoFile`.

#### FR-GH-009: Formatted Markdown README Renderer & Strict Sanitization
- **Description:** The system SHALL render repository `README.md` documents formatted as clean, sanitized HTML with syntax highlighting and external link interception.
- **Priority:** P1 (Mandatory)
- **Security Guardrail (SEC-05):** All rendered HTML MUST be strictly sanitized using the Rust `ammonia` sanitization crate or client-side `DOMPurify` prior to injection into the DOM, stripping malicious script tags, iframes, inline event handlers (`onload`, `onerror`), and unsafe URI schemes (`javascript:`), resolving the legacy XSS vulnerability.
- **Legacy Reference:** `search_service.go:GetRepoReadme`, `github/client.go:GetRepoReadme`.

#### FR-GH-010: Safe Repository Deletion with Permission Diagnostic
- **Description:** The system SHALL allow repository deletion ONLY after:
  1. Validating that the active token possesses the `delete_repo` OAuth scope.
  2. Displaying a destructive confirmation modal requiring the user to type the repository name or click explicit confirmation.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `search_service.go:DeleteRepo`, `HUONG_DAN.txt:Section 3.3`.

#### FR-GH-011: Token Diagnostic & Rate-Limit Telemetry Report
- **Description:** The system SHALL provide a diagnostics view displaying:
  - Active Token Scopes (`repo`, `delete_repo`, `admin:org`, `workflow`, `gist`).
  - Rate Limits: Limit, Remaining, Reset Timestamp (countdown timer), Used calls.
  - Account Profile: Disk storage used, 2FA status, Public/Private repository count.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `system_service.go:GetTokenReport`, `github/client.go:GetTokenReport`.

#### FR-GH-012: Comprehensive Submission History & Recent Push Tracker
- **Description:** The system SHALL persist complete submission and push records in local SQLite storage (`rusqlite`) and surface the most recent record on the home dashboard with a one-click "Copy Link" and "Open in Browser" action.
- **Priority:** P1 (Mandatory)
- **Legacy Limitation Resolution:** Resolves the legacy defect where `history.json` only stored a single `LastPushed` record and erased all previous submission logs. The rebuilt engine maintains an indexed relational history table storing: `id`, `profile_id`, `repo_name`, `repo_url`, `local_path`, `commit_sha`, `commit_msg`, `pushed_at`, `status`, and `file_count`.
- **Analytics & Filtering:** Users can filter submission history by date range, profile, or repository name, and view submission activity heatmaps.
- **Legacy Reference:** `history/tracker.go:SaveLastPushed`, `GetLastPushed`.

#### FR-GH-013: Branch Protection Rules Inspector & Push Guard
- **Description:** The system SHALL inspect GitHub branch protection rules for the active repository branch and enforce pre-push compliance.
- **Priority:** P1 (Mandatory)
- **Processing:**
  - Queries `GET /repos/{owner}/{repo}/branches/{branch}/protection`.
  - Parses protection settings: Required Pull Request reviews (`required_approving_review_count`), Required status checks (CI/CD passing), Enforce linear history, Restrict pushes.
  - Displays GitHub Primer branch protection badge in repository header.
  - If direct pushing is prohibited by branch rules, the system intercepts the push attempt, explains the rule clearly, and prompts the user to create a Pull Request instead (`FR-PR-001`).

---

### 3.3 Multi-Repo Batch Operations (FR-BAT)

#### FR-BAT-001: Multi-Repository Directory Scanner
- **Description:** The system SHALL allow users to select or drop a parent directory containing multiple sub-repositories or select multiple discrete directories simultaneously.
- **Priority:** P1 (Major New Feature)
- **Processing:** The system scans child folders, checks for `.git` or project markers, and populates a batch queue list with repository status (clean, dirty, ahead, behind, unlinked).

#### FR-BAT-002: Batch Queue Manager & Filter Controls
- **Description:** The system SHALL allow users to inspect the batch list, select all or subset of repositories, customize individual or uniform commit messages, and specify target branches.
- **Priority:** P1 (Major New Feature)

#### FR-BAT-003: Concurrency-Controlled Worker Pool
- **Description:** Batch execution SHALL run via an asynchronous worker pool managed by a Tokio `Semaphore` with user-configurable concurrency limits (default: 4 concurrent operations, range: 1 to 16).
- **Priority:** P1 (Major New Feature)
- **Processing:** Prevents system resource exhaustion, local file lock contention, and GitHub API secondary rate limits.

#### FR-BAT-004: Dual-Level Progress Visualization
- **Description:** The batch UI SHALL display:
  1. **Global Aggregated Progress:** Overall percentage, completed / total count, elapsed time, ETA.
  2. **Per-Repository Progress Card:** Individual progress bar, current phase (`Init` -> `Stage` -> `Commit` -> `Push`), and real-time status icon (Queued, Running, Success, Failed, Skipped).
- **Priority:** P1 (Major New Feature)

#### FR-BAT-005: Isolated Fault Tolerance & Partial Failure Handling
- **Description:** An error or merge conflict in one repository during batch execution SHALL NOT terminate or interrupt the execution of remaining repositories in the queue.
- **Priority:** P1 (Major New Feature)
- **Processing:** Failed repositories are flagged with detailed error explanations, while successful pushes continue to completion. A batch summary report is generated upon completion.

#### FR-BAT-006: Batch Clone & Pull Synchronization
- **Description:** The system SHALL support batch cloning of multiple repositories selected from GitHub account search or pasted URL lists, with automatic workspace directory creation and concurrent downloading.
- **Priority:** P2 (High)

---

### 3.4 Smart Conflict Resolution Visualizer (FR-CRV)

#### FR-CRV-001: Pre-Flight Divergence & Conflict Detection
- **Description:** Prior to executing any push or sync operation, the system SHALL check if the remote tracking branch contains upstream commits not present in the local branch (`ahead > 0 && behind > 0`).
- **Priority:** P1 (Major New Feature)
- **Processing:** If divergence is detected, the system suspends the push and presents the user with conflict resolution options rather than performing an unsafe force-push.

#### FR-CRV-002: 3-Way Merge Inspection Engine
- **Description:** When conflicting files exist between local `HEAD` and remote branch, the system SHALL parse the 3-way merge objects (Base Common Ancestor, Ours / Local, Theirs / Remote) using `git2::Index` conflict entries.
- **Priority:** P1 (Major New Feature)

#### FR-CRV-003: Side-by-Side Visual Diff & Conflict Editor
- **Description:** The system SHALL provide a graphical conflict editor with side-by-side or unified diff viewing:
  - Left Pane: Current Local Changes ("Ours")
  - Right Pane: Incoming Remote Changes ("Theirs")
  - Bottom / Result Pane: Live Merged Output with syntax highlighting
- **Priority:** P1 (Major New Feature)

#### FR-CRV-004: Quick Resolution Decision Actions
- **Description:** For every conflicted chunk or file, the system SHALL offer one-click resolution buttons:
  - **Accept Ours (Local):** Overwrites conflict with local state.
  - **Accept Theirs (Remote):** Overwrites conflict with remote state.
  - **Accept Both (Combine):** Appends local after remote or vice-versa.
  - **Manual Edit:** Allows direct interactive editing within the result pane.
- **Priority:** P1 (Major New Feature)

#### FR-CRV-005: Merge Finalization & Automatic Commit
- **Description:** Once all conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) are resolved, the system SHALL mark files resolved (`git2::Index::add`), generate a clean Merge Commit, and push the unified history safely to the remote.
- **Priority:** P1 (Major New Feature)

---

### 3.5 Cloud AI Copilot & Multi-Provider Engine (FR-AI)

#### FR-AI-001: Cloud Multi-Provider LLM Architecture (Zero Local LLM Overhead)
- **Description:** The system SHALL provide an intelligent, cloud-only AI Copilot powered by premier Cloud LLM providers, completely excluding heavy local LLM dependencies (Ollama / Llama-cpp) to preserve an ultra-lean (<30MB RAM) runtime footprint.
- **Supported Cloud LLM Providers:**
  - **Google Gemini API:** Gemini 2.0 Flash (high speed default) and Gemini 1.5 Pro.
  - **OpenAI API:** GPT-4o and GPT-4o-mini.
  - **Anthropic Claude API:** Claude 3.5 Sonnet and Claude 3.5 Haiku.
  - **Groq Cloud API:** Llama-3.3-70B-Versatile and Mixtral-8x7B (ultra-low latency <1.5s).
  - **OpenRouter API:** Universal proxy supporting 200+ curated models.
- **Priority:** P1 (Mandatory)
- **Processing:** Standardized `CloudLlmClient` trait in Rust with model-specific JSON-RPC/REST streaming, token budget enforcement, and prompt optimization.

#### FR-AI-002: Multi-Cloud Dynamic Failover & Rate-Limit Resilience
- **Description:** The system SHALL implement an automatic multi-cloud failover pipeline. If the user's primary Cloud provider encounters rate limiting (HTTP 429), capacity errors (HTTP 503), or network timeouts, the engine SHALL automatically route the request to a configured secondary cloud provider with exponential backoff and zero user disruption.
- **Priority:** P1 (Mandatory)

#### FR-AI-003: Diff-Aware Intelligent Commit Message Generator
- **Description:** The system SHALL analyze staged Git diffs (`git diff --cached`) and automatically generate conventional commit messages (`feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`) summarizing code changes concisely.
- **Priority:** P1 (Major New Feature)
- **Input:** Staged diff text (truncated safely to model context limit).
- **Output:** Structured commit title and bulleted description.

#### FR-AI-004: Pull Request Description & Summary Generator
- **Description:** The system SHALL inspect the commit delta between the current branch and `main`/`master` and generate a complete Markdown Pull Request draft with summary, changes list, and test instructions.
- **Priority:** P2 (High)

#### FR-AI-005: Automated Error Diagnosis & Fix Recommendation
- **Description:** When a Git operation (push, clone, merge) fails with an error:
  1. The system captures stderr and exit codes.
  2. The Stephen AI floating badge pulses with an alert icon (`alert-circle`).
  3. Clicking the badge opens the chat with an immediate, plain-language Vietnamese/English explanation of the error (e.g. invalid credentials, large file rejected >100MB, merge conflict, non-fast-forward) and provides the exact button or command to fix it.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `frontend/src/services/ChatService.js:triggerDiagnostics`, `push_service.go:49-51`.

#### FR-AI-006: Static Rule Zero-Token Intent Filter
- **Description:** To optimize performance, eliminate unnecessary API costs, and guarantee instant responses, the system SHALL evaluate incoming user queries against local regex intent rules prior to querying any LLM:
  - Greetings (`xin chào`, `hello`, `hi`, `chào stephen`): Instant friendly response.
  - Off-Topic queries (sports, politics, celebrity trivia): Polite redirect to Git topics.
  - Developer & Project queries ("Ai phát triển?", "tác giả"): Instant response with author credentials and GitHub repo link.
  - Feature help queries ("voice", "clone", "naming pattern"): Instant contextual instructions.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `frontend/src/services/chatRules.js:1-74`.

#### FR-AI-007: Agent Action Execution Engine
- **Description:** Stephen AI responses containing structured agent action tokens SHALL trigger interactive GUI action prompts:
  - `[AGENT_ACTION:clone:url=<url>]` -> Switches to Clone tab and pre-fills target URL.
  - `[AGENT_ACTION:push:folder=<path>]` -> Switches to Push tab and loads specified directory.
  - `[AGENT_ACTION:navigate:tab=<tab_id>]` -> Smoothly navigates to target tab.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `frontend/src/services/chatRules.js:95-123`, `ChatService.js`.

#### FR-AI-008: Voice Command Recognition & Intent Parser
- **Description:** The system SHALL accept microphone voice input (via Web Speech API or local whisper inference), transcribe speech, parse intent using regex rules or LLM fallback, and automatically execute the target navigation, push, clone, or search action.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `ai_service.go:ProcessVoiceCommand` (lines 187-403).

#### FR-AI-009: Smart Directory Matcher
- **Description:** When the user issues a voice or text command like "Đẩy bài buổi 15 bài 2", the system SHALL automatically scan parent directories of the last pushed project, Desktop, and Documents to find and select matching folder structures (e.g., `Session15/Ex02`, `Buoi15/Bai02`).
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `ai_service.go:FindMatchingDirectory` (lines 405-544).

#### FR-AI-010: Context-Aware Workspace Telemetry Injection
- **Description:** AI chat prompts SHALL include the active workspace context: selected repository, current branch, clean/dirty status, last pushed repository, and platform OS, enabling accurate contextual advice.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `ai_service.go:GetAppContext` (lines 152-185).

#### FR-AI-011: Automated AI Code Review & Security Smells Scanner
- **Description:** The system SHALL provide an on-demand AI Code Review action on staged changes or commits. The AI analyzes code quality, detects accidental token/credential leaks, flags performance anti-patterns, and highlights common beginner bugs (e.g., resource leaks, uncaught exceptions, concurrency data races).
- **Priority:** P2 (High)
- **Input:** Unified diff or file AST context.
- **Output:** Categorized findings (Security, Bug Risk, Style/Clean Code) with inline actionable suggestion snippets.

#### FR-AI-012: Interactive Diff Explanation & Teaching Assistant Mode
- **Description:** The system SHALL allow users to highlight any hunk in the Diff Inspector or Conflict Visualizer and request an immediate plain-language Vietnamese/English explanation of what the change does and why conflicts or errors may occur.
- **Priority:** P2 (High)
- **Output:** Educational explanation tailored for learners and instructors.

---

### 3.6 Multi-Account & Multi-Profile Management (FR-ACC)

#### FR-ACC-001: Multi-Profile Identity Architecture
- **Description:** The system SHALL allow users to create and manage multiple profiles (e.g., "Personal GitHub", "Work GitHub", "University/School", "Client A").
- **Priority:** P1 (Major New Feature)

#### FR-ACC-002: Independent Per-Profile Credentials & Identity
- **Description:** Each profile SHALL maintain independent:
  - Git Committer Identity: `user.name` and `user.email`.
  - Authentication Token: Securely stored under distinct OS Keyring service keys.
  - Default Target: User account vs. Specific Organization name.
  - Default Naming Pattern and Commit Message template.
- **Priority:** P1 (Major New Feature)

#### FR-ACC-003: Instant Global Context Switcher
- **Description:** A profile dropdown in the top sidebar header SHALL switch active profile credentials and repository visibility across all tabs instantly without requiring an application restart.
- **Priority:** P1 (Major New Feature)

#### FR-ACC-004: Per-Repository Profile Binding
- **Description:** The system SHALL allow associating a local repository with a specific profile. When opening or selecting that repository, the application SHALL automatically activate the corresponding profile and credentials.
- **Priority:** P2 (High)

#### FR-ACC-005: Secure Profile Export & Import (Encrypted Backup)
- **Description:** The system SHALL allow exporting non-sensitive profile configurations or encrypted credential archives protected by a user-supplied passphrase for easy migration across machines.
- **Priority:** P2 (High)

---

### 3.7 Background Git Watcher & Auto-Sync Engine (FR-WAT)

#### FR-WAT-001: Low-Overhead Filesystem Monitoring
- **Description:** The system SHALL provide an optional background folder watcher using the native `notify` crate (ReadDirectoryChangesW on Windows, inotify on Linux, FSEvents on macOS).
- **Priority:** P1 (Major New Feature)

#### FR-WAT-002: Intelligent Configurable Debounce Engine
- **Description:** Filesystem change events SHALL pass through a debounce timer (configurable between 5 seconds and 120 seconds, default: 15 seconds) to ensure multi-file edits (e.g., compilation output, file saves) settle before triggering Git operations.
- **Priority:** P1 (Major New Feature)

#### FR-WAT-003: Automated Stage & Commit on Idle
- **Description:** Once the debounce timer elapses without further filesystem modifications, the watcher SHALL automatically stage modified files and create a timestamped or AI-summarized commit.
- **Priority:** P1 (Major New Feature)

#### FR-WAT-004: Safe Auto-Sync (Pull-Merge-Push)
- **Description:** If enabled in settings, the system SHALL attempt an automatic safe push to the remote repository. If upstream remote commits are detected, the system safely pauses, issues a non-intrusive desktop notification, and requests manual review to prevent conflicts.
- **Priority:** P1 (Major New Feature)

#### FR-WAT-005: System Tray & Quiet Background Operation
- **Description:** When the auto-sync watcher is active, the application window MAY be minimized to the OS System Tray. The tray icon displays active watcher status and provides a quick context menu to pause, trigger sync now, or open the UI.
- **Priority:** P2 (High)

---

### 3.8 Settings, Customization & System Utilities (FR-SET)

#### FR-SET-001: Hardware-Backed Native OS Keyring Storage
- **Description:** All secret tokens (GitHub PAT, GitHub OAuth token, Cloud AI API keys) SHALL be stored exclusively using the operating system credential store (`keyring-rs`).
- **Priority:** P1 (Mandatory Security Requirement)
- **Legacy Replacement:** Completely replaces plaintext storage in `config.json` and `.env`.
- **Keyring Identifiers:**
  - Service: `com.gitupload.desktop`
  - Keys: `github_token:<profile_id>`, `ai_api_key:<provider_id>`.

#### FR-SET-002: Non-Sensitive Configuration Persistence
- **Description:** Non-sensitive preferences (theme, language, auto-push toggle, naming pattern, default commit patterns) SHALL be stored in `%APPDATA%\GitUpload\preferences.json` (Windows) or `~/.config/gitupload/preferences.json` (Linux/macOS) formatted with human-readable JSON indentation.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `config/config.go:GetConfigPath`.

#### FR-SET-003: Tri-State Modern Theme Engine
- **Description:** The system SHALL provide three visual theme modes:
  - **Dark Mode:** High-contrast slate/zinc palette (default).
  - **Light Mode:** Crisp, glare-free light palette.
  - **System Synchronized:** Automatically switches theme matching the host OS appearance settings.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `frontend/src/services/ConfigService.js:setupThemeSystem`.

#### FR-SET-004: Dual-Language Internationalization (i18n)
- **Description:** The entire user interface, instructions, tooltips, dialogs, and AI assistant dialogues SHALL support **Vietnamese (Tiếng Việt - Default)** and **English (US)** with instantaneous runtime switching.
- **Priority:** P1 (Mandatory)

#### FR-SET-005: Single-Instance Mutex Enforcement
- **Description:** The system SHALL permit only one active running instance per user session. Launching a secondary instance SHALL display a lightweight warning dialog and focus the existing instance window before exiting immediately.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `main.go:checkSingleInstance` (lines 18-40).
- **Implementation:** Rust `named_lock` crate or native Win32 `CreateMutexW("Local\\GitUploadSingleInstanceMutex")`.

#### FR-SET-006: Global Drag & Drop Engine
- **Description:** The system SHALL support seamless drag-and-drop of directories and single files from Windows Explorer / Finder.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `main.go:86-89` (`EnableFileDrop: true`), `frontend/src/main.js:38-40`.
- **Safety:** Automatically intercepts webview drop events to prevent navigation to `about:blank#blocked`.

#### FR-SET-007: Embedded Offline User Guide
- **Description:** The system SHALL embed the complete user documentation (`HUONG_DAN.txt` and interactive guide) into the binary at compile time via `include_str!`. The guide is parsed dynamically into horizontal tabs for instant offline reading.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `app.go:24-25` (`//go:embed HUONG_DAN.txt`), `system_service.go:GetLocalGuideHTML`.

#### FR-SET-008: Global External Link Interceptor
- **Description:** Any hyperlink clicked within the webview pointing to `http://` or `https://` SHALL be intercepted and opened in the user's default system browser via native OS handlers (`open::that`), preventing webview navigation.
- **Priority:** P1 (Mandatory)
- **Legacy Reference:** `system_service.go:OpenBrowser`, `frontend/src/main.js:99-109`.

---

### 3.9 Pull Request (PR) Lifecycle Management (FR-PR)

#### FR-PR-001: Pull Request Creation & Fork Tracking
- **Description:** The system SHALL allow users to create GitHub Pull Requests directly from the desktop interface without navigating to GitHub web.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Selects base repository (upstream parent repository if the active repo is a fork) and target branch (e.g., `main`).
  - Selects compare repository and head branch (current feature branch).
  - Pre-fills PR title and description with an on-demand "Generate with Cloud AI" action (`FR-AI-004`).
  - Supports toggling between standard PR and "Draft PR" (`draft: true`).
  - Calls `POST /repos/{owner}/{repo}/pulls` via GitHub REST API v3.

#### FR-PR-002: PR Diff Inspection & Changed Files Tree
- **Description:** The system SHALL render a dedicated Pull Request overview displaying:
  - Changed files tree with addition/deletion metrics (`+X / -Y`).
  - Side-by-side and unified syntax-highlighted diff viewers styled with the GitHub Primer Design System.
  - Commits list associated with the Pull Request with author avatars and SHAs.
- **Priority:** P1 (Major New Feature)

#### FR-PR-003: Interactive PR Reviews & Inline Comments
- **Description:** The system SHALL allow users to review Pull Requests:
  - Browse inline discussion threads and file comments.
  - Submit official review decisions: `APPROVE`, `REQUEST_CHANGES`, or `COMMENT` with Markdown summary.
  - Calls `POST /repos/{owner}/{repo}/pulls/{pull_number}/reviews`.
- **Priority:** P2 (High)

#### FR-PR-004: Multi-Strategy Pull Request Merging
- **Description:** When authorized, the user SHALL be able to merge open Pull Requests directly from the app.
- **Priority:** P1 (Major New Feature)
- **Merge Strategies Supported:**
  1. **Create a merge commit:** Standard Git merge (`merge_method: merge`).
  2. **Squash and merge:** Combines all PR commits into a single unified commit (`merge_method: squash`).
  3. **Rebase and merge:** Re-applies individual commits linearly onto the base branch (`merge_method: rebase`).
- **Processing:** Calls `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge` and provides optional checkbox to automatically delete the head branch post-merge.

#### FR-PR-005: Pull Request Search & Filter Dashboard
- **Description:** The system SHALL provide a centralized Pull Request dashboard to search and filter PRs across personal and organization repositories by state (`open`, `closed`, `all`), author, and review status.
- **Priority:** P2 (High)

---

### 3.10 Native Git Hooks Engine (FR-HOK)

#### FR-HOK-001: Zero-Dependency Native Hooks Architecture
- **Description:** The system SHALL provide a built-in, standalone Git Hooks manager that replaces external JavaScript/Node-based tools like Husky or Lefthook with 100% native Rust logic.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Installs lightweight shell/batch dispatcher wrappers into `.git/hooks/` (`commit-msg`, `pre-commit`, `pre-push`).
  - Wrapper scripts execute the `rust-gitupload` core binary with sub-millisecond execution times.
  - Requires **ZERO Node.js, npm, npx, Python, or external runtime dependencies**.

#### FR-HOK-002: commit-msg Conventional Commits Linter
- **Description:** The `commit-msg` hook SHALL validate commit messages against the Conventional Commits 1.0.0 specification before finalizing any commit.
- **Priority:** P1 (Major New Feature)
- **Validation Rules:**
  - Format: `<type>(<scope>?): <description>`
  - Permitted types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
  - Header length limit: Maximum 72 characters.
  - Subject case: Must not start with an uppercase letter; must not end with a period.
  - Returns clear, colorized error hints on stdout explaining exactly why validation failed.

#### FR-HOK-003: pre-commit Safety & Quality Guard
- **Description:** The `pre-commit` hook SHALL execute automated safety and quality checks on staged files prior to commit creation.
- **Priority:** P1 (Major New Feature)
- **Guard Verifications:**
  - **Large File Blocker:** Rejects files >50MB with a prompt to enable Git LFS (`FR-GIT-017`), preventing GitHub 100MB hard rejection.
  - **Secret & Credential Leak Scanner:** Scans staged diffs for private keys (`-----BEGIN RSA PRIVATE KEY-----`), GitHub tokens (`ghp_`), AWS access keys (`AKIA`), and password fields.
  - **Formatting Check:** Optionally runs project-configured linters (`cargo fmt`, `prettier`, `gofmt`).

#### FR-HOK-004: pre-push Verification & Branch Protection Guard
- **Description:** The `pre-push` hook SHALL verify that the target branch is not a protected main branch before initiating network transfer and verify that all test suites pass.
- **Priority:** P2 (High)

#### FR-HOK-005: Declarative Configuration (.githooks.toml) & GUI Manager
- **Description:** Git hooks SHALL be configured declaratively via a `.githooks.toml` file checked into the root of the repository, and managed visually through a "Git Hooks" configuration panel in the GUI.
- **Priority:** P2 (High)

---

### 3.11 Interactive Git Graph & Branch Tree Visualizer (FR-GRF)

#### FR-GRF-001: Topological DAG Commit Graph & Swimlanes
- **Description:** The system SHALL compute and render an interactive, color-coded Directed Acyclic Graph (DAG) representing the complete Git commit topology across all local and remote branches.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Traverses commits using `git2::Revwalk` with topological sorting (`git2::Sort::TOPOLOGICAL | git2::Sort::TIME`).
  - Calculates branch swimlane indices and assign distinct theme-harmonized colors (blue, green, purple, orange, teal, pink).
  - Renders smooth curved Bezier spline connectors representing branching and merge lines.
  - Distinct node visual markers: Standard Commit (solid circle), Merge Commit (double circle), Stash Node (dashed square), Tag Flag (badge pill), Branch Ref (rounded label), and `HEAD` pointer (highlighted pulse ring).

#### FR-GRF-002: Virtualized High-Performance Canvas Rendering
- **Description:** The Git Graph viewer SHALL utilize virtualized DOM or HTML5 Canvas rendering to support repositories with up to 100,000 commits without UI frame drops, maintaining a consistent **60+ FPS** scrolling performance.
- **Priority:** P1 (Performance Critical)
- **Processing:** Only visible commits within the viewport bounding box (plus 50 row pre-fetch buffer) are rendered and laid out, dynamically querying the Rust backend in 100-commit micro-pages.

#### FR-GRF-003: Commit Inspector & Multi-File Diff Viewer
- **Description:** Selecting any commit node in the graph SHALL instantly display a detailed inspector pane:
  - Commit Metadata: Full SHA, Parent SHAs, Author/Committer name & email, Gravatar/GitHub avatar, precise microsecond timestamp, and cryptographic GPG/SSH verification status (`[Verified]` badge).
  - Full Commit Message with conventional commit formatting and issue references (`#123` auto-linked).
  - Modified Files List with line addition/deletion metrics (`+X / -Y`) and change type badges (`[M]odified`, `[A]dded`, `[D]eleted`, `[R]enamed`).
  - Integrated syntax-highlighted Diff Viewer with side-by-side / inline toggle.
- **Priority:** P1 (Major New Feature)

#### FR-GRF-004: Contextual Direct Graph Actions
- **Description:** Right-clicking or accessing the contextual menu for any commit or branch in the graph SHALL provide direct operational actions:
  - **Checkout Commit / Branch:** Checks out the selected revision (`detached HEAD` warning dialog if raw commit).
  - **Create Branch / Tag at Commit:** Opens a dialog pre-filled with the selected commit SHA to create lightweight or annotated tags and branches.
  - **Cherry-Pick Commit:** Applies the selected commit onto the current `HEAD` via `git2::Repository::cherrypick`.
  - **Revert Commit:** Creates an inverse revert commit via `git2::Repository::revert`.
  - **Reset (Soft / Mixed / Hard):** Resets active branch `HEAD` to the target commit with safe confirmation prompt explaining the difference between soft (keeps index & working tree), mixed (keeps working tree), and hard (destroys uncommitted changes).
  - **Interactive Rebase Helper:** Initiates interactive rebase sequence (squash, edit, drop, reword) with visual reordering.
- **Priority:** P1 (Major New Feature)

#### FR-GRF-005: Multi-Branch Comparison & Ahead/Behind Matrix
- **Description:** The system SHALL allow selecting any two branches (e.g., `feature/login` vs `origin/main`) and display:
  - Divergence metrics: `Ahead by X commits`, `Behind by Y commits`.
  - Commit list unique to each side.
  - Consolidated file tree diff between the two branch heads.
- **Priority:** P2 (High)

---

### 3.12 GitHub CLI (`gh`) Full Lifecycle Manager & Auth Bridge (FR-GHCLI)

#### FR-GHCLI-001: GitHub CLI Dual-Environment Detection
- **Description:** On startup and on demand, the system SHALL check for the presence and version of the official GitHub CLI (`gh` / `gh.exe`).
- **Priority:** P1 (Major New Feature)
- **Detection Algorithm:**
  1. Checks if `gh` exists on the system `PATH` (`gh --version`).
  2. Checks for a local portable binary inside the application sandbox: `%APPDATA%\GitUpload\bin\gh.exe` (Windows) or `~/.local/share/gitupload/bin/gh` (Linux/macOS).
  3. Validates minimum required version (>= 2.40.0) and records detection source (`SystemPATH` or `AppSandbox`).

#### FR-GHCLI-002: One-Click / Silent Sandbox Auto-Installer
- **Description:** If `gh` is not detected on the host machine, the system SHALL offer a one-click automated installation that requires **NO administrator/UAC privileges**:
  - Automatically queries the GitHub Releases API for `cli/cli` to locate the latest stable release.
  - Downloads the official Windows `gh_<version>_windows_amd64.zip` (or ARM64/Linux/macOS tarball).
  - Computes and verifies the cryptographic SHA-256 checksum against the official release manifest.
  - Extracts the standalone `bin/gh.exe` executable into `%APPDATA%\GitUpload\bin\`.
  - Adds the sandboxed directory to the application's runtime subprocess environment `PATH`.
  - Alternatively, if `winget` is detected, allows triggering `winget install --id GitHub.cli` with user consent.
- **Priority:** P1 (Major New Feature)

#### FR-GHCLI-003: Seamless Background Auto-Updater
- **Description:** The system SHALL periodically check (daily or weekly, user configurable) for newer releases of the GitHub CLI.
- **Priority:** P2 (High)
- **Processing:** If a newer release is published, the system downloads and updates the sandboxed `gh.exe` atomically via temporary file replacement (`gh.new.exe` -> atomic rename).

#### FR-GHCLI-004: Dual-Auth Zero-Friction Credential Bridge
- **Description:** If the user has already authenticated with the GitHub CLI on their machine (`gh auth login`), the system SHALL provide a seamless one-click credential import bridge:
  - Invokes `gh auth token` safely through the Tokio asynchronous process runner.
  - Extracts the active GitHub token without requiring the user to open a browser, navigate to Developer Settings, or manually copy/paste a PAT.
  - Encrypts and saves the imported token directly into the native Windows Credential Manager / OS Keyring (`FR-SET-001`).
  - If the token is refreshed or switched in `gh`, the application detects the change and synchronizes seamlessly.
- **Priority:** P1 (Major High-Productivity Feature)

---

### 3.13 GitHub Actions, Issues, Releases & Developer Automations (FR-ACT, FR-ISS, FR-REL, FR-AUT)

#### FR-ACT-001: GitHub Actions CI/CD Workflow Monitor
- **Description:** The system SHALL provide a dedicated CI/CD monitoring interface for GitHub Actions workflows for the active repository.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Queries `GET /repos/{owner}/{repo}/actions/runs` (or `gh run list --json`).
  - Displays workflow runs list: Name, Commit, Trigger Event (`push`, `pull_request`, `workflow_dispatch`), Author, Status (`completed`, `in_progress`, `queued`), and Conclusion (`success`, `failure`, `cancelled`).
  - Interactive Step Log Streamer: Clicking a run streams the real-time build logs with full ANSI escape color code parsing.
  - Run Management: One-click buttons to `Rerun Failed Jobs`, `Rerun All Jobs`, or `Cancel Workflow Run`.

#### FR-ISS-001: In-App GitHub Issue Tracker
- **Description:** The system SHALL provide an integrated Issue Management dashboard to browse, search, and manage GitHub Issues without leaving the application.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Lists open and closed issues with label badges, assignees, milestone, and comment count.
  - Issue Detail View with Markdown description and comment timeline.
  - Create Issue: Form with title, labels, assignees, and a "Draft with Cloud AI" prompt to generate clear bug reports or feature requests.
  - Close / Reopen Issue actions with status updates reflected instantly.

#### FR-REL-001: GitHub Release Manager with AI-Generated Changelogs
- **Description:** The system SHALL allow users to create and publish official GitHub Releases with automated semantic changelogs.
- **Priority:** P1 (Major New Feature)
- **Processing:**
  - Inspects Git commit history between the target tag and previous release tag.
  - Calls Cloud AI (`FR-AI-001`) to categorize commits into conventional changelog sections: `🚀 Features`, `🐛 Bug Fixes`, `📚 Documentation`, `⚡ Performance`, and `⚠️ Breaking Changes`.
  - Supports dragging-and-dropping compiled binary assets (e.g. `.exe`, `.zip`, `.msi`) to upload directly to GitHub Release assets (`POST /repos/{owner}/{repo}/releases/{id}/assets`).
  - Flags draft vs. pre-release status.

#### FR-AUT-001: GitHub Gist Manager
- **Description:** The system SHALL allow users to create and manage GitHub Gists for quick code snippet sharing.
- **Priority:** P2 (High)
- **Processing:** Supports single or multi-file Gists, public or secret visibility, and provides instant "Copy Raw URL" and "Copy Clone URL" actions.

#### FR-AUT-002: Fork & Upstream Synchronization Engine
- **Description:** The system SHALL provide one-click Fork management and upstream sync:
  - Detects if the current repository is a fork of an upstream parent repository (`parent` object in GitHub API).
  - Queries upstream divergence: displays how many commits the fork is behind upstream `main`.
  - One-click `Sync Fork` executes `POST /repos/{owner}/{repo}/merge-upstream` (or fast-forward fetch/merge) cleanly.

#### FR-AUT-003: Smart Developer Automations
- **Description:** The system SHALL provide productivity automations to streamline everyday Git workflows:
  - **Auto-Stash on Sync / Re-Push:** If uncommitted working directory edits exist when a pull, re-push, or branch switch is initiated, the engine automatically stashes changes (`git stash create`), executes the Git operation, and re-applies the stash (`git stash pop`), prompting only if a conflict occurs.
  - **Conventional Branch Naming Engine:** Offers standardized branch naming patterns for student assignments (e.g., `hw/SS{session}-EX{exercise}-{student_id}`, `feature/{topic}`, `fix/{issue_id}`).
  - **Dry-Run Pre-Push Simulation:** Simulates the remote push before executing network transfers (`git push --dry-run`), detecting rejected pushes, branch protection blocks, or missing credentials without altering remote state.
- **Priority:** P1 (Major New Feature)

---

## 4. External Interface Requirements

### 4.1 User Interfaces - GitHub Primer Design System Fidelity (UI/UX)
The application user interface SHALL provide 1:1 visual fidelity with the official GitHub web platform, adopting the GitHub Primer Design System standards:
- **Design Framework & Tokens:** Implements GitHub Primer design tokens (colors, typography, border radiuses, spacing scales, and focus states).
- **Iconography:** Exclusively utilizes official GitHub Octicons SVG iconography (e.g., `repo`, `git-branch`, `git-pull-request`, `git-commit`, `git-compare`, `shield-check`, `file-directory`, `organization`, `check-circle-fill`, `x-circle-fill`).
- **Comprehensive Theme Suite:**
  1. **GitHub Dark (Default):** Canvas background `#0d1117`, surface `#161b22`, border `#30363d`, text `#e6edf3`.
  2. **GitHub Dark Dimmed:** Canvas background `#22272e`, surface `#2d333b`, border `#444c56`, text `#adbac7`.
  3. **GitHub Light:** Canvas background `#ffffff`, surface `#f6f8fa`, border `#d0d7de`, text `#1f2328`.
  4. **GitHub High Contrast:** Pure black `#010409` canvas with high-contrast `#7a828e` borders for accessibility.
- **Authentic GitHub-Style Components:**
  - **Repository Header Bar:** Owner/Repository breadcrumb link, Public/Private visibility pill, Star / Fork / Watch buttons with live counters, and Branch selector dropdown with branch protection shield indicator.
  - **GitHub-Fidelity File Tree Explorer:** Standard GitHub table layout displaying file/directory icon, name, latest commit message preview, author avatar, and relative timestamp (`2 hours ago`).
  - **Syntax-Highlighted Diff Viewer:** Switchable **Side-by-Side (Split)** and **Unified (Inline)** diff view featuring dual line-number gutters, green tint (`#2ea04326`) for additions, red tint (`#f8514926`) for deletions, and interactive expand/collapse hunk controls.
  - **Commit & Verification Badges:** Cryptographic `[Verified]` green signature badge and CI check status indicators (green checkmark / red x / pending yellow circle).
  - **Pull Request Timeline:** Complete PR conversation stream, review status box (Approved / Changes Requested), and big green "Merge Pull Request" dropdown button.
  - **Stephen AI Primer Drawer:** Bottom-right floating assistant matching GitHub Copilot chat styling with resizable pane and expandable quick actions.

### 4.2 Hardware & System Interfaces
- **CPU:** x86_64 or ARM64 architecture (2+ cores recommended).
- **RAM:** Minimum 512MB free system memory.
- **Display:** 1024x600 minimum; supports HiDPI scaling up to 200% on Windows/macOS Retina displays.

### 4.3 Software & Native Operating System Interfaces
- **Windows APIs:**
  - `Advapi32.dll` / `wincred.h`: Windows Credential Manager for secure token storage.
  - `Kernel32.dll`: `CreateMutexW` for single-instance enforcement.
  - `User32.dll`: `MessageBoxW` for pre-initialization fatal warnings.
  - `Shell32.dll`: Detached process launching with `CREATE_NO_WINDOW`.
- **Linux APIs:** `libsecret-1` / D-Bus Secret Service API for Keyring.
- **macOS APIs:** Security Framework (`SecKeychain`) for Keyring.

### 4.4 Communications & Network Protocol Interfaces
- **GitHub API:** HTTPS REST API v3 and GraphQL API v4 (`https://api.github.com`) using TLS 1.3. Connection pooling with Keep-Alive.
- **Git Smart Transfer Protocol:** HTTPS Git transfer (`https://github.com/<owner>/<repo>.git`) and optional SSH (`git@github.com:...`) with Git LFS transfer endpoints.
- **Cloud LLM Protocol:** Secure HTTPS REST endpoints with SSE streaming:
  - Google Gemini API (`https://generativelanguage.googleapis.com/v1beta`)
  - OpenAI API (`https://api.openai.com/v1`)
  - Anthropic Claude API (`https://api.anthropic.com/v1`)
  - Groq Cloud API (`https://api.groq.com/openai/v1`)
  - OpenRouter API (`https://openrouter.ai/api/v1`)

---

## 5. Rigorous Non-Functional Requirements (NFR-xxx)

### 5.1 Performance Requirements (NFR-PERF)

#### NFR-PERF-001: Ultra-Low Idle Memory Footprint
- **Target:** **< 30 MB Private Working Set RAM** when the application is idle in the background.
- **Verification:** Measured using Windows Process Explorer / Resource Monitor 60 seconds after application startup. Compared against legacy Go/Wails baseline (~150 MB).

#### NFR-PERF-002: Bounded Peak Load Memory
- **Target:** **< 100 MB RAM** during heavy operations (e.g. batch pushing 10 repositories or generating 50MB commit diffs).
- **Mechanism:** Direct streaming buffers, zero-copy diff iterators, and automatic buffer reuse.

#### NFR-PERF-003: Sub-100ms Cold Startup Latency
- **Target:** Cold start to fully interactive window presentation in **< 100 milliseconds** on modern SSD hardware.
- **Verification:** High-resolution monotonic timer measured from `main()` entry to frontend `DOMContentLoaded` event.

#### NFR-PERF-004: 60+ FPS Zero-Stutter User Interface
- **Target:** The GUI rendering loop MUST maintain 60+ frames per second during long-running Git push or clone operations.
- **Mechanism:** Git network I/O and disk pack compression execute exclusively on Tokio background worker threads; progress data is throttled to 60Hz event broadcasts over Tauri IPC.

#### NFR-PERF-005: Low Network Latency via HTTP Connection Pooling
- **Target:** GitHub API queries MUST reuse existing TCP/TLS connections via a long-lived `reqwest::Client` connection pool, reducing roundtrip latency by up to 60%.

#### NFR-PERF-006: High-Throughput Batch Concurrency
- **Target:** Batch processing of 10 average-sized repositories (5-10MB each) SHALL complete within 25 seconds on a standard 50Mbps internet connection.

#### NFR-PERF-007: Ultra-Fast Cloud AI Response Latency
- **Target:** Commit message generation SHALL respond in **< 1.5 seconds** when routed via Groq or Gemini 2.0 Flash. Pull Request descriptions SHALL respond in **< 3.0 seconds**.
- **Efficiency:** Pure cloud execution imposes **0 MB local RAM overhead** and **0 MB VRAM overhead** on the user's host machine.

---

### 5.2 Security & Credential Vault Requirements (NFR-SEC)

#### NFR-SEC-001: Zero Plaintext Secrets on Storage
- **Requirement:** Under NO circumstances shall GitHub Personal Access Tokens, OAuth refresh tokens, or AI API keys be written to unencrypted disk files (`.env`, `config.json`, logs).
- **Enforcement:** Mandatory routing through native OS Credential Manager (`keyring-rs`). Attempting to serialize config structs with token fields into JSON is prohibited at compile time.

#### NFR-SEC-002: In-Memory Authentication & Secret Redaction in Telemetry
- **Requirement:** Access tokens MUST NEVER be passed in plaintext command-line arguments or URLs (`https://<token>@github.com`), which exposed tokens to local process monitors in the legacy system (SEC-03).
- **Authentication Security:** Remote authentication with `libgit2` MUST utilize in-memory credential callbacks (`git2::Cred::userpass_plaintext` or SSH keys) residing in volatile protected memory (`secrecy::SecretString`). If the CLI fallback is invoked, credentials MUST be provided via secure environment-scoped `GIT_ASKPASS` pipes or Git credential helpers, never via URL arguments.
- **Telemetry Redaction:** All logs, console displays, error toasts, and event streams MUST automatically redact any string matching access token patterns (`ghp_...`, `github_pat_...`, `Bearer ...`, `https://.*@github.com`).
- **Enforcement:** Centralized redaction filter in logger pipeline replaces tokens with `***REDACTED***`.

#### NFR-SEC-003: Safe Push Enforcement by Default
- **Requirement:** Destructive `git push --force` is strictly forbidden by default. Fast-forward and safe merge pushes are enforced. Force-push requires an explicit opt-in setting guarded by an alert dialog.

#### NFR-SEC-004: Subprocess Sandboxing & Argument Escaping
- **Requirement:** CLI fallback invocations MUST pass arguments as discrete array parameters directly to Windows APIs without intermediary shell interpretation (`cmd.exe /c`), preventing command injection.

#### NFR-SEC-005: Safe Input & Path Sanitization
- **Requirement:** All repository names, branch names, and path inputs MUST be strictly sanitized against directory traversal (`..`, `\`, `/`, null bytes) and invalid GitHub characters.

---

### 5.3 Reliability, Fault Tolerance & Recovery (NFR-REL)

#### NFR-REL-001: Atomic Workspace Operations & Rollback
- **Requirement:** If a Git operation fails halfway (e.g., network timeout during pack upload), the system MUST roll back temporary lock files (`.git/index.lock`) and leave the working directory in a clean, recoverable state.

#### NFR-REL-002: Clean Lock Management & Process Cancellation
- **Requirement:** User cancellation via the "Hủy" button MUST immediately abort network sockets and release file system locks within 500 milliseconds.

#### NFR-REL-003: Graceful Network Degradation
- **Requirement:** If internet connection drops during operation, the application MUST display clear, localized error diagnostics and allow immediate retry without restarting the application.

#### NFR-REL-004: Crash-Free State Persistence
- **Requirement:** Application preferences and history files MUST be written using atomic write-and-rename semantics (`tempfile` + `fs::rename`) to prevent file corruption in case of sudden power loss.

---

### 5.4 Portability & Cross-Platform Compliance (NFR-PORT)

#### NFR-PORT-001: Windows-First Optimization
- **Requirement:** Full native support for Windows 10/11 x86_64 and ARM64 architectures, compiled with MSVC toolchain, integrated with Windows Credential Manager and native Windows notification system.

#### NFR-PORT-002: Full Linux & macOS Compatibility
- **Requirement:** Clean build and execution on Linux (x86_64/ARM64) using WebKitGTK and D-Bus Secret Service, and macOS (Apple Silicon/Intel) using Cocoa WebKit and Keychain Services.

#### NFR-PORT-003: Path Separator Normalization
- **Requirement:** All internal path handling MUST utilize Rust `std::path::PathBuf` and normalize to forward slashes (`/`) when interacting with Git remotes or URLs.

---

### 5.5 Usability, Internationalization & Accessibility (NFR-USE)

#### NFR-USE-001: Bi-Lingual Localization
- **Requirement:** 100% of UI labels, buttons, console logs, error diagnostics, and help documents MUST support both Vietnamese (`vi-VN`) and English (`en-US`). Default language on first launch is Vietnamese.

#### NFR-USE-002: High-Contrast Accessibility
- **Requirement:** Text contrast ratios MUST meet WCAG 2.1 AA standards (minimum 4.5:1 for normal text, 3:1 for large text). Focus indicators must be clearly visible for keyboard navigation.

#### NFR-USE-003: Keyboard Shortcuts & Hotkeys
- **Requirement:** Common workflows SHALL support global and in-app hotkeys:
  - `Ctrl + Enter`: Trigger Push / Action
  - `Ctrl + F`: Jump to Search
  - `Ctrl + ,`: Open Settings
  - `Ctrl + Shift + C`: Copy recent repo link
  - `Esc`: Close open modal / cancel current action

---

### 5.6 Maintainability, Modularity & Testability (NFR-MNT)

#### NFR-MNT-001: Modular Workspace Architecture
- **Requirement:** The codebase MUST be structured as a Cargo workspace with distinct, decoupled crates/modules:
  - `gitupload-core`: Pure domain models, errors (`thiserror`), interfaces, and naming regex utilities.
  - `gitupload-security`: Native OS Keyring (`keyring-rs`), token log sanitization, and memory-safe secret protection (`secrecy`/`zeroize`).
  - `gitupload-storage`: Embedded SQLite persistence with WAL mode, schema migrations, and push history audit trail.
  - `gitupload-git`: Dual-engine Git abstraction (`libgit2` in-process + CLI fallback), worker pool, Git LFS, and cryptographic signing.
  - `gitupload-hooks`: Pure Rust native Git hooks manager (Husky/Lefthook equivalent), Conventional Commits linter, pre-commit/pre-push guards.
  - `gitupload-ghcli`: GitHub CLI (`gh`) full lifecycle manager, portable SHA256 sandbox installer, auto-updater, and dual-auth bridge.
  - `gitupload-github`: GitHub REST/GraphQL API client, OAuth device flow, Organizations support, and Pull Request (PR) lifecycle engine.
  - `gitupload-ai`: Pure Cloud AI multi-provider client (Gemini, OpenAI, Claude, Groq, OpenRouter) with zero local LLM overhead.
  - `gitupload-watcher`: Filesystem monitoring (`notify`) and debounced auto-sync queue.
  - `gitupload-app`: Tauri v2 application entrypoint, single-instance mutex, GitHub Primer UI bindings, IPC commands, and event streaming bridges.

#### NFR-MNT-002: Test Coverage Benchmark
- **Requirement:** The core domain logic (`gitupload-core`, `gitupload-github`, `gitupload-ai`) MUST achieve a minimum of **80% automated unit and integration test coverage**.

---

## 6. Legacy Codebase Feature Traceability Matrix

This matrix verifies that **100% of legacy functions, workflows, and frontend components** in `F:\Build\JavaToolGitAutoUpload\g-gitupload` are fully accounted for, preserved, and modernized in the new specification:

| Legacy Source Component | Legacy Go Function / File | Modern Functional Requirement (FR) | Modern Rust Implementation Details |
| :--- | :--- | :--- | :--- |
| **Push Service** | `StartPush()` (`push_service.go:32`) | `FR-GIT-001`, `FR-GIT-002`, `FR-GIT-007` | Tokio async command, `libgit2` staging & safe push |
| **Push Service** | `cleanPath()` (`push_service.go:207`) | `FR-GIT-002`, `NFR-SEC-005` | `PathBuf` canonicalization and quote stripping |
| **Push Service** | `resolveRepoName()` (`push_service.go:215`) | `FR-GIT-004` | Pattern substitution engine (`formatRepoName`) |
| **Push Service** | `resolveCommitMsgForFile()` (`push_service.go:333`) | `FR-GIT-003`, `FR-GIT-006` | Regex variable replacement `{filename}`, `{session}` |
| **Push Service** | `CancelPush()` (`push_service.go:193`) | `FR-GIT-015`, `NFR-REL-002` | Tokio `CancellationToken` instant socket abort |
| **Git Runner** | `CheckGitAvailable()` (`git/runner.go:18`) | `FR-GIT-001`, `FR-GIT-010` | In-process `libgit2` check + optional CLI probe |
| **Git Runner** | `SplitByNewlineOrCR()` (`git/runner.go:28`) | `FR-GIT-008` | Stream buffer scanner handling `\r` and `\n` |
| **Git Runner** | `PushHomework()` (`git/runner.go:112`) | `FR-GIT-001` - `FR-GIT-007` | `libgit2` repository init, commit, push without CLI |
| **Git Runner** | `ensureEnvIgnored()` (`git/runner.go:191`) | `FR-GIT-005` | Auto-appending safety patterns to `.gitignore` |
| **Clone Service** | `StartClone()` (`clone_service.go:28`) | `FR-GIT-009`, `FR-GIT-008` | `git2::build::RepoBuilder` with progress callbacks |
| **Clone Service** | `DetectIDEs()` (`clone_service.go:106`) | `FR-GIT-010` | Windows Registry / filesystem scanner for IDEs |
| **Search Service** | `SearchRepos()` (`search_service.go:17`) | `FR-GH-005`, `FR-GH-006` | `reqwest` async GitHub REST v3 search & sort |
| **Search Service** | `DeleteRepo()` (`search_service.go:47`) | `FR-GH-010` | Guarded `DELETE` request with scope verification |
| **Search Service** | `GetRepoContents()` (`search_service.go:83`)| `FR-GH-007` | Remote directory tree explorer via GitHub API |
| **Search Service** | `GetRepoReadme()` (`search_service.go:101`) | `FR-GH-009` | Formatted Markdown HTML retrieval |
| **Search Service** | `UpdateRepoFile()` (`search_service.go:137`)| `FR-GH-008` | In-app editor file commit and push |
| **AI Service** | `SendChatMessage()` (`ai_service.go:57`) | `FR-AI-001`, `FR-AI-002` | Multi-cloud provider client (Gemini, OpenAI, Claude, Groq, OpenRouter) with dynamic failover |
| **AI Service** | Commit Generator & PR Drafter | `FR-AI-003`, `FR-AI-004` | Diff-aware commit message and PR description generator |
| **AI Service** | Error Diagnostics & Badge | `FR-AI-005` | Auto-detect push/clone errors and guide remediation |
| **AI Service** | Static Intent Rule Engine (`chatRules.js`) | `FR-AI-006` | Zero-token instant regex responses for greetings/queries |
| **AI Service** | Agent Action Dispatcher (`chatRules.js`) | `FR-AI-007` | Interactive GUI action triggers (`[AGENT_ACTION:...]`) |
| **AI Service** | `ProcessVoiceCommand()` (`ai_service.go:333`)| `FR-AI-008` | Regex rules intent parser with LLM fallback |
| **AI Service** | `FindMatchingDirectory()` (`ai_service.go:406`)| `FR-AI-009` | Smart filesystem fuzzy scanner for sessions/exercises |
| **AI Service** | `GetAppContext()` (`ai_service.go:162`) | `FR-AI-010` | Contextual system & workspace state injector |
| **AI Service** | Code Review & Smell Scanner | `FR-AI-011` | Automated static/LLM code quality & secret scan |
| **AI Service** | Interactive Diff & Conflict Explainer | `FR-AI-012` | Teaching assistant mode for diffs & conflict hunks |
| **Git Engine (New)** | Re-push & Incremental Sync | `FR-GIT-016` | Existing `.git` detection, remote tracking inspection, `force-with-lease` |
| **Git Engine (New)** | Git LFS Asset Management | `FR-GIT-017` | Detection of files >50MB, `.gitattributes` configuration, LFS pointer upload |
| **Git Engine (New)** | `.gitignore` Generator | `FR-GIT-018` | Built-in offline multi-tag generator for 100+ language/framework templates |
| **Git Engine (New)** | Cryptographic Commit Signing | `FR-GIT-019` | GPG and SSH commit signing with GitHub `[Verified]` green badge |
| **Git Graph (New)** | Interactive Commit DAG & Tree | `FR-GRF-001` - `FR-GRF-005` | Visual swimlanes, virtualized 60 FPS canvas, cherry-pick, reset, rebase helper |
| **GitHub CLI (New)**| `gh` Full Lifecycle Manager | `FR-GHCLI-001` - `FR-GHCLI-004` | Detection, sandbox auto-install (SHA256), auto-update, `gh auth token` vault bridge |
| **CI/CD Monitor (New)**| GitHub Actions Workflow Monitor | `FR-ACT-001` | Live ANSI log streaming, step-by-step progress, rerun failed jobs |
| **Issue Tracker (New)**| In-App GitHub Issue Manager | `FR-ISS-001` | Browse, create (AI-drafted), label, and close/reopen issues |
| **Release Mgr (New)** | GitHub Release & AI Changelog | `FR-REL-001` | Semantic AI changelog generator, tag creator, binary asset uploader |
| **Developer Tools (New)**| Gist, Fork Sync & Automations | `FR-AUT-001` - `FR-AUT-003` | Public/secret Gists, fork upstream sync, auto-stash, dry-run simulation |
| **Git Hooks (New)**| Native Hooks Manager | `FR-HOK-001` - `FR-HOK-005`| Zero-dependency Husky/Lefthook in Rust; Conventional Commits & secret guard |
| **GitHub PR (New)**| Pull Request Lifecycle | `FR-PR-001` - `FR-PR-005` | Create PR, draft toggle, review comments, and multi-strategy merge |
| **GitHub Org (New)**| Organizations & Protection | `FR-GH-003`, `FR-GH-013` | `/user/orgs` context switching, org repo creation, branch protection guard |
| **System Service** | `GetSettings()`, `SaveSettings()` | `FR-SET-001`, `FR-SET-002` | Native OS Keyring for secrets + JSON for prefs |
| **System Service** | `GetTokenReport()` (`system_service.go:105`)| `FR-GH-011` | Scopes, rate limits, and 2FA diagnostic report |
| **System Service** | `GetLocalGuideHTML()` (`system_service.go:117`)| `FR-SET-007` | Embedded `include_str!("HUONG_DAN.txt")` parser |
| **History Tracker**| `GetLastPushed()`, `SaveLastPushed()` | `FR-GH-012` | Atomic JSON history store in app config dir |
| **Main Runtime** | `checkSingleInstance()` (`main.go:18`) | `FR-SET-005` | Named Mutex / `named_lock` single instance lock |
| **Main Runtime** | `showMessageBox()` (`main.go:42`) | `FR-SET-005` | Native Win32 `MessageBoxW` fallback |
| **Frontend UI** | `PushTab` & DropZone (`index.html:52-153`)| `FR-GIT-002`, `FR-SET-006` | GitHub Primer styling, drag-and-drop file upload |
| **Frontend UI** | `CloneTab` & IDE Actions (`index.html:156`)| `FR-GIT-009`, `FR-GIT-010` | Quick IDE launcher buttons on clone success |
| **Frontend UI** | `SearchTab` & Details (`index.html:234`)| `FR-GH-005` - `FR-GH-008` | Server-side pagination, file editor, README viewer |
| **Frontend UI** | `ReportTab` (`index.html:552-565`) | `FR-GH-011` | Visual diagnostic cards for Token & Rate limits |
| **Frontend UI** | `GuideTab` (`index.html:567-581`) | `FR-SET-007` | Tabbed Vietnamese documentation viewer |
| **Frontend UI** | `Stephen AI Chat` (`index.html:627-667`)| `FR-AI-005` - `FR-AI-007` | Resizable AI drawer, error badges, agent actions |
| **Frontend UI** | `VoiceAssistant` (`index.html:38-41`) | `FR-AI-008` | Voice recognition and automated workflow execution |

---

## 7. Verification, Quality Assurance & Acceptance Criteria

### 7.1 Formal Acceptance Criteria

| Req ID | Acceptance Test Description | Success Metric / Verification Criteria |
| :--- | :--- | :--- |
| **FR-GIT-002** | Drag-and-drop a single Markdown assignment file | File is copied to isolated temp workspace; parent directory unchanged; valid repo pushed. |
| **FR-GIT-007** | Attempt to push local branch when remote contains newer commits | Push is safely suspended; user alerted with conflict resolution prompt; 0 remote commits lost. |
| **FR-GIT-016** | Select existing `.git` folder and re-push after local edits | Remote tracking inspected; ahead/behind counters displayed; incremental push succeeds without re-init. |
| **FR-BAT-003** | Batch push 8 repositories simultaneously | Tokio Semaphore restricts concurrent operations to 4; each repo completes with isolated log. |
| **FR-CRV-003** | Simulate 3-way merge conflict on `README.md` | Conflict visualizer opens; shows Ours vs. Theirs; allows one-click resolution and clean merge. |
| **FR-HOK-002** | Execute `git commit -m "bad message"` with hook enabled | Conventional Commits hook rejects commit in <10ms; prints colorized format requirement hint. |
| **FR-PR-004** | Merge a Pull Request via "Squash and merge" | Pull Request merged on GitHub via API; commits condensed into single clean entry on target branch. |
| **FR-GH-003** | Switch active context to an Organization | Repositories filter to organization; new repository created under organization namespace. |
| **FR-AI-002** | Simulate primary cloud rate limit (HTTP 429) during commit generation | Engine automatically fails over to secondary cloud provider (Groq/Gemini); responds in <1.5s. |
| **FR-ACC-003** | Switch active profile from "Personal" to "Work" | Token, Git identity, and repository search views update immediately without app restart. |
| **FR-WAT-003** | Save edits to 3 files in watched directory | Debounce timer waits 15 seconds after last save; automatically stages and commits changes cleanly. |
| **FR-GRF-001** | Open 10,000+ commit repository in Git Graph viewer | Virtualized canvas renders at **60+ FPS** without UI freezing; swimlanes and commit dots display smoothly. |
| **FR-GHCLI-002**| Launch app on clean system without `gh.exe` installed | One-click installation silently downloads official portable `gh.exe` into sandbox, verifies SHA256 in <5s. |
| **FR-GHCLI-004**| Click "Sync Auth from GitHub CLI" button | Automatically extracts token via `gh auth token` into Windows Credential Manager without manual PAT input. |
| **FR-ACT-001** | Inspect active GitHub Actions run | Streams live ANSI build logs in real-time; offers one-click rerun for failed workflow steps. |
| **FR-REL-001** | Create release for repository tag `v2.0.0` | Cloud AI generates categorized changelog from commit range; binary assets uploaded to GitHub Release. |
| **FR-AUT-003** | Trigger re-push with uncommitted working directory edits | Auto-stash securely stashes local changes, executes push, and pops stash without data loss. |
| **NFR-PERF-001**| Measure application memory usage at idle | Private Working Set memory consumes **< 30 MB RAM** on 64-bit Windows 11. |
| **NFR-PERF-003**| Measure cold start time on SSD | Application window reaches interactive ready state in **< 100 milliseconds**. |
| **NFR-SEC-001** | Inspect entire filesystem and AppData directory for tokens | **Zero instances** of plaintext GitHub tokens or AI keys in any `.json`, `.env`, or log file. |

### 7.2 Automated Testing Strategy
1. **Unit Tests:** All string sanitizers, filename parsers, slug generators, and commit pattern formatters tested with edge cases (empty strings, special Unicode characters, path traversal attempts).
2. **Integration Tests with Temp Repositories:** Automated test suite spins up local Git repositories using `tempfile`, executes git2 operations, and asserts clean git status.
3. **Mock GitHub API Server:** Mock HTTP server (using `wiremock` crate) verifies correct GitHub REST API request headers, payload serialization, and rate-limit handling.
4. **Keyring Abstraction Tests:** Test doubles verify credential retrieval and fallback error messaging across test environments.

---

*End of Software Requirements Specification (SRS-RUST-GITUPLOAD-V2.2)*
