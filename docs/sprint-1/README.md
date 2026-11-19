# Sprint 1 Engineering Execution Plan — Master Guide
**Document Identifier:** SPRINT1-MASTER-EXECUTION-PLAN  
**Project:** GitUpload Desktop Suite (Rust / Tauri v2 / Primer UI)  
**Version:** 2.2.0  
**Timeline:** 25 Working Days (5 Phased Iterations)  
**Status:** Approved for Implementation  
**Audience:** Core Engineers, Subagents, Swarm Workers, Verifiers (@verifier, @coordinator)  
**Reference Standards:** IEEE Std 830-1998, ISO/IEC/IEEE 12207, ISO/IEC/IEEE 29148  

---

## 1. Sprint 1 Mission & Strategic Objectives

**Sprint 1** encompasses the complete end-to-end engineering implementation of the modernized **GitUpload Desktop Suite**. It transitions the project from architectural specifications ([01_LEGACY_AUDIT.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/01_LEGACY_AUDIT.md), [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md), [03_SRS.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/03_SRS.md), [04_MIGRATION_ROADMAP.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/04_MIGRATION_ROADMAP.md)) into production-ready, compile-clean, and battle-tested software.

### Primary Engineering Objectives:
1. **Total Modular Workspace Construction:** Implement 10 decoupled Rust crates under `crates/` and a modern GitHub Primer React frontend under `ui/`.
2. **Zero-Plaintext Security & Native OS Keyring:** Eradicate all plaintext token storage (`.env`, `config.json`). Implement hardware/DPAPI-backed Windows Credential Manager integration via `keyring` and memory zeroization.
3. **Dual Git Engine with Interactive DAG Graph:** High-speed in-process `libgit2` engine with fallback CLI subprocesses isolated in Windows Job Objects. Interactive 60+ FPS Git DAG graph with topological swimlane routing and direct contextual actions.
4. **Zero-Dependency Native Git Hooks:** Pure-Rust hook runner enforcing Conventional Commits 1.0.0, staged secret scanning, and branch protection without requiring Node.js or npm.
5. **Sandboxed GitHub CLI Lifecycle & Dual Auth Bridge:** Zero-admin auto-installer with SHA256 verification and seamless extraction of active tokens into the Windows Keyring.
6. **Pure Cloud AI Acceleration:** High-speed streaming LLM orchestrator (Gemini 2.0/1.5, GPT-4o, Claude 3.5 Sonnet, Groq) with zero local model bloat, guaranteeing **< 28MB idle RAM** and **< 100ms cold startup**.

---

## 2. Phase Index & Detailed Specifications Map

Sprint 1 is strictly partitioned into 5 phased technical iterations. Each phase has a dedicated, exhaustive engineering specification:

| Phase | Days | Specification Document | Primary Target Crates & Modules | Key Deliverables & Milestones |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 1** | Days 1–5 | [phase-1-core-security-storage-hooks.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-1-core-security-storage-hooks.md) | `gitupload-core`, `gitupload-security`, `gitupload-storage`, `gitupload-hooks` | Domain models, `thiserror` error trees, Windows Keyring vault, SQLite WAL schema, legacy data auto-migrator, and pure-Rust Git hooks (`commit-msg`, `pre-commit`, `pre-push`). |
| **Phase 2** | Days 6–10 | [phase-2-git-engine-graph-safesync.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-2-git-engine-graph-safesync.md) | `gitupload-git` | Dual Git Engine (`libgit2` + Job Object CLI), `git2::Revwalk` DAG graph builder, swimlane routing, Re-push from existing repos, safe sync (`--force-with-lease`), Git LFS, and GPG signing. |
| **Phase 3** | Days 11–15 | [phase-3-tauri-ipc-ghcli-primer-ui.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-3-tauri-ipc-ghcli-primer-ui.md) | `gitupload-ghcli`, `gitupload-app`, `ui/` | GitHub CLI (`gh`) sandbox installer, SHA256 checksums, Dual Auth bridge, Tauri v2 IPC command registry, Win32 mutex, GitHub Primer UI, and virtualized 60 FPS Git Graph canvas. |
| **Phase 4** | Days 16–20 | [phase-4-cloud-ai-actions-prs-releases.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-4-cloud-ai-actions-prs-releases.md) | `gitupload-ai`, `gitupload-github`, `gitupload-watcher` | Pure Cloud AI orchestrator (Gemini/GPT/Claude/Groq), GitHub Actions live ANSI stream, in-app Issue Tracker, AI Release Notes generator, PR lifecycle, and multi-repo batch concurrency. |
| **Phase 5** | Days 21–25 | [phase-5-qa-benchmarks-packaging.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-1/phase-5-qa-benchmarks-packaging.md) | All crates, `tests/`, `benches/` | Ephemeral Git test harnesses, Criterion performance benchmarks (<28MB RAM, <100ms startup), Clippy `-D warnings` audit, `#![forbid(unsafe_code)]` scan, and NSIS/MSI production packaging (<15MB). |

---

## 3. Crate Responsibility & Dependency Matrix

The 10 modular crates maintain strict acyclic layering:

```
+----------------------------------------------------------------------------------------------------+
|                                    gitupload-app (Tauri v2 Desktop Shell)                          |
+----+--------+--------+--------+--------+--------+--------+--------+--------+-----------------------+
     |        |        |        |        |        |        |        |        |
     |        |        |        |        |        |        |        |        v
     |        |        |        |        |        |        |        +---> gitupload-watcher
     |        |        |        |        |        |        v
     |        |        |        |        |        +---> gitupload-ai (Pure Cloud AI)
     |        |        |        |        v
     |        |        |        +---> gitupload-github (REST / GraphQL)
     |        |        v
     |        +---> gitupload-ghcli (CLI Sandbox & Dual Auth)
     |        |
     |        +---> gitupload-hooks (Native Pure-Rust Git Hooks)
     |        |
     |        +---> gitupload-git (Dual Engine & DAG Revwalk)
     |        |        |
     |        v        v
     +---> gitupload-security (Windows Keyring Vault & Token Sanitizer)
     |        |
     |        v
     +---> gitupload-storage (SQLite WAL Persistence)
     |        |
     v        v
   gitupload-core (Domain Models, Error Tree, Traits, Cancellation)
```

---

## 4. Definition of Ready (DoR)

Before an engineer or subagent begins implementing any task within a phase:
1. **Architectural Alignment:** The task must map directly to an explicit Task ID (e.g. `TASK-P1-01`) in the respective Phase Specification.
2. **Dependency Resolution:** All prerequisite crates, modules, or traits must be committed and pass `cargo check`.
3. **Data Contract Frozen:** Serde data models, IPC payload envelopes, and database schemas must match the specification without ad-hoc alterations.
4. **Security Grounding:** No task may introduce plaintext secret storage, unredacted log emissions, or unconstrained unsafe blocks.

---

## 5. Definition of Done (DoD)

A task or phase is considered complete only when:
1. **Compile Cleanliness:** Code compiles with zero warnings under `cargo check --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
2. **Format Adherence:** Code passes `cargo fmt --check`.
3. **Automated Verification:** 100% of unit tests and integration test fixtures for the phase pass (`cargo test -p <crate_name>`).
4. **Zero-Unsafe Invariant:** All crates strictly observe `#![forbid(unsafe_code)]` (except bounded Win32 Named Mutex / Job Object initializations in `gitupload-app` and `gitupload-git::cli`).
5. **Memory & Performance SLAs:** Micro-benchmarks verify adherence to Sprint 1 SLAs (< 28MB idle RAM, < 100ms startup).
6. **Documentation Integrity:** All public types and functions have rustdoc comments; cross-links use valid `file:///` URLs.
7. **Swarm Verification:** Formal sign-off posted to the swarm message board by `@verifier`.
