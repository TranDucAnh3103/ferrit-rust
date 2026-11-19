# AGENTS.md — GitUpload Agent Collaboration Manual
**Version:** 2.2.0  
**Scope:** AI Agents, Subagents, Swarm Workers, and Human Engineers working on `rust-gitupload`  
**Status:** Mandatory Governance Specification  

---

## 1. Project Mission & Architectural Overview

The **GitUpload** project (`rust-gitupload`) is an enterprise-grade, memory-efficient, and secure Rust desktop application powered by **Tauri v2** and **Tokio**. It completely replaces the legacy Go/Wails prototype (`g-gitupload`) with an authentic **GitHub-fidelity desktop client** built on the **GitHub Primer Design System**.

### 1.1 The 10 Modular Workspace Crates

The codebase strictly isolates responsibilities across 10 modular crates in `rust-gitupload/crates/`:

| Crate | Responsibility | Key Dependencies / APIs |
| :--- | :--- | :--- |
| **`gitupload-core`** | Shared domain models, PR/Org/Hook types, `thiserror` error tree, cancellation tokens, traits. | `serde`, `thiserror`, `tokio-util`, `chrono` |
| **`gitupload-security`** | Windows Credential Manager integration, `secrecy`/`zeroize` memory hygiene, token sanitization. | `keyring`, `secrecy`, `zeroize`, `regex` |
| **`gitupload-storage`** | Embedded SQLite with WAL mode, schema DDL, versioned migrations, relational query repository. | `rusqlite`, `rusqlite_migration`, `r2d2` |
| **`gitupload-git`** | Dual Git engine (in-process `libgit2` + CLI fallback), DAG revwalk graph visualizer, diff streaming. | `git2`, `tokio::process`, Windows Job Objects |
| **`gitupload-hooks`** | Native pure-Rust Git hooks runner (`commit-msg`, `pre-commit`, `pre-push`) with zero external runtime deps. | `regex`, `glob`, `toml` |
| **`gitupload-ghcli`** | GitHub CLI (`gh`) detection, portable SHA256 auto-installer, sandbox runner, Dual Auth Bridge. | `sha2`, `zip`, `semver`, `reqwest` |
| **`gitupload-github`** | GitHub REST & GraphQL client (Orgs, PRs, Workflow Actions, Branch Protection, LFS, Gists). | `reqwest`, `eventsource-stream` |
| **`gitupload-ai`** | Pure Cloud AI orchestrator (Google Gemini 2.0/1.5, OpenAI GPT-4o, Claude 3.5, Groq, OpenRouter). | `reqwest`, `eventsource-stream` |
| **`gitupload-watcher`** | Filesystem change monitoring with debounced auto-sync queues. | `notify`, `tokio::sync::mpsc` |
| **`gitupload-app`** | Tauri v2 desktop application shell, Primer UI IPC commands, streaming event emitters, Win32 mutex. | `tauri`, `tauri-plugin-*`, `windows-sys` |

---

## 2. Non-Negotiable Safety Guardrails

Every AI agent and contributing engineer MUST strictly observe these non-negotiable rules:

### 2.1 `#![forbid(unsafe_code)]`
- All crates MUST specify `#![forbid(unsafe_code)]` at their crate root (`src/lib.rs` / `src/main.rs`).
- The single allowed exception is the Win32 Named Mutex and Job Object initialization in `gitupload-app` and `gitupload-git::cli`, which must be wrapped in strictly bounded, commented `unsafe` blocks using `windows-sys`.

### 2.2 Zero-Plaintext Credential Policy (SEC-01)
- **NEVER** write GitHub Personal Access Tokens (PAT), OAuth bearer tokens, or AI API keys to plaintext files (`.env`, `config.json`, or SQLite text columns).
- All sensitive credentials MUST be stored inside the **OS Credential Vault** (`keyring` crate backed by Windows Credential Manager).
- In-memory representations of secrets MUST be wrapped in `secrecy::SecretString` with `zeroize::Zeroize` on drop.

### 2.3 Automated Token Log Sanitization (SEC-02)
- **NEVER** print, log, or stream unredacted tokens.
- All tracing subscribers and Tauri IPC event emitters MUST pass messages through `gitupload_security::sanitizer::sanitize_log_message`.
- GitHub PAT patterns (`ghp_*`, `github_pat_*`) and embedded HTTPS remote URLs (`https://***@github.com`) must be redacted before disk writes or UI transmissions.

### 2.4 Pure Cloud AI — No Local LLM Bloat
- **DO NOT** add or suggest local LLM dependencies (Ollama, Llama-cpp, Candle).
- The AI subsystem is strictly **Pure Cloud AI** via Gemini, OpenAI, Claude, Groq, and OpenRouter to preserve an ultra-lean `< 28 MB` idle RAM footprint.

---

## 3. Rust Coding Standards & Idioms

### 3.1 Error Handling with `thiserror`
- Do NOT use `anyhow` in core library crates (`gitupload-core`, `gitupload-git`, `gitupload-security`, etc.).
- Every domain crate must expose strongly typed error enums derived via `thiserror::Error`.
- Never use `.unwrap()` or `.expect()` in production paths. Propagate errors via `?` or map them cleanly into `AppError` / `IpcErrorPayload`.

### 3.2 Async Runtime & Concurrency Patterns
- **Async Runtime**: Use **Tokio** exclusively (`tokio = "1.41"`).
- **Blocking Git Operations**: `libgit2` C calls and heavy filesystem traversals MUST be executed inside `tokio::task::spawn_blocking` to prevent UI thread lockups.
- **Micro-Batched Event Streams**: High-throughput log output from Git operations or workflow runs must be micro-batched (30ms debounce / 60Hz rate limit) before emitting across Tauri IPC.
- **Cancellation**: Long-running operations must accept `tokio_util::sync::CancellationToken` and check it at every stage or callback.
- **Batch Throttling**: Multi-repo concurrent tasks must be bounded with `tokio::sync::Semaphore` (max 3 concurrent jobs).

### 3.3 Clippy & Format Enforcement
- All Rust code must pass `cargo fmt --check` without errors.
- All code must pass `cargo clippy --workspace --all-targets -- -D warnings`.

---

## 4. Conventional Commits & Git Workflow

### 4.1 Commit Message Standard
All commits must strictly follow **Conventional Commits 1.0.0**:
```
<type>(<scope>): <short imperative description>

[optional body]

[optional footer(s)]
```
- **Allowed Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- **Allowed Scopes**: `core`, `security`, `storage`, `git`, `hooks`, `ghcli`, `github`, `ai`, `watcher`, `app`, `ui`.
- **Examples**:
  - `feat(hooks): add native conventional commit message linter`
  - `fix(ghcli): resolve portable zip extraction path on Windows`
  - `refactor(git): optimize DAG revwalk swimlane allocator`

### 4.2 Branch Naming Conventions
- Feature branches: `feat/<feature-name>` (e.g. `feat/git-graph-dag`)
- Bug fix branches: `fix/<issue-name>` (e.g. `fix/keyring-lookup-timeout`)
- Refactor branches: `refactor/<module>` (e.g. `refactor/cloud-ai-sse`)
- Student homework: `feat/ss<XX>-ex<YY>-<student_id>`

---

## 5. Agent Workflow & Verification Protocol

When working on any task:
1. **Consult Rules**: Read the applicable rules in [.agent/rules/](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/):
   - [01-architecture-principles.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/01-architecture-principles.md)
   - [02-rust-coding-standards.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/02-rust-coding-standards.md)
   - [03-security-and-credentials.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/03-security-and-credentials.md)
   - [04-tauri-ipc-conventions.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/04-tauri-ipc-conventions.md)
   - [05-testing-and-verification.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/05-testing-and-verification.md)
   - [06-ui-ux-design-system.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/06-ui-ux-design-system.md)
2. **Execute Locally**: Compile and verify with `cargo check --workspace` and `cargo test --workspace`.
3. **Verify Crate Boundaries**: Run the skill [verify-crate-boundaries](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/skills/verify-crate-boundaries/SKILL.md) and [run-git-tests](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/skills/run-git-tests/SKILL.md) before committing.
4. **Cross-Link Deliverables**: Link all markdown references using valid `file:///` URLs with forward slashes.

---

## 6. Project Deliverables & Specification Map

| Document | Purpose |
| :--- | :--- |
| [README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/README.md) | Project Overview, Primer UI Design & Quickstart |
| [01_LEGACY_AUDIT.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/01_LEGACY_AUDIT.md) | Legacy Go/Wails Deep Audit & Technical Debt Inventory |
| [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) | C4 Component Architecture & 10-Crate System Design |
| [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md) | IEEE 830 Software Requirements Specification |
| [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md) | 25-Day 5-Phase Implementation & Migration Guide |
| [docs/sprint-1/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/README.md) | Granular Sprint 1 Phased Execution Plan (Tasks P1-P5 & ACs) |
| [docs/sprint-2/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/README.md) | Granular Sprint 2 Plan: Multi-Remote, 3-Way Merge, SSH Agent, Heatmap |
| [docs/sprint-3/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-3/README.md) | Granular Sprint 3 Plan: AI DevSecOps, Semantic Search, Stale Branches |
| [docs/sprint-4/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/README.md) | Granular Sprint 4 Plan: macOS Apple Silicon, Linux, Minisign Updater |
| [docs/sprint-5/README.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md) | Granular Sprint 5 Plan: WASM Plugins, Webhooks, Headless CLI, Marketplace |
