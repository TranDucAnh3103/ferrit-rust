# Sprint 2 Engineering Execution Plan — Master Guide
## Team Collaboration, Multi-Remote Mirroring & Interactive 3-Way Merge
**Document Identifier:** SPRINT2-MASTER-EXECUTION-PLAN  
**Project:** GitUpload Desktop Suite (Rust / Tauri v2 / Primer UI)  
**Target Release:** v2.3.0  
**Timeline:** 25 Working Days (5 Phased Iterations)  
**Status:** Approved for Implementation  
**Audience:** Core Engineers, Subagents, Swarm Workers, Verifiers (@verifier, @coordinator)  
**Reference Standards:** IEEE Std 830-1998, ISO/IEC/IEEE 12207, ISO/IEC/IEEE 29148  

---

## 1. Sprint 2 Mission & Strategic Objectives

Following the successful foundation, core Git engine, and desktop shell delivered in Sprint 1, **Sprint 2** scales the GitUpload Desktop application into an enterprise-ready **team collaboration, multi-cloud repository mirroring, and conflict resolution platform**.

### Primary Engineering Objectives:
1. **Multi-Remote Synchronization & Mirroring (`phase-1`):** Simultaneous parallel push and fetch across multiple hosting providers (**GitHub, GitLab, Bitbucket, self-hosted Gitea**), with per-remote credential mapping, isolated timeouts, and aggregate progress reporting.
2. **Interactive 3-Way Visual Merge Conflict Resolver (`phase-2`):** Monaco-based 3-pane visual merge interface (Current / Base / Incoming / Result), gutter conflict markers, syntactic diff highlighting, 1-click chunk resolution ("Accept Current", "Accept Incoming", "Accept Both"), and conflict-free commit generation.
3. **Pure-Rust Native SSH Agent & Key Management (`phase-3`):** Zero-dependency ED25519 and RSA 4096 key generator, in-memory SSH agent daemon with Windows Named Pipe interface, and automated public key upload to GitHub/GitLab REST APIs.
4. **Team Analytics, Submission Heatmaps & Velocity (`phase-4`):** Relational analytics engine in SQLite tracking student commit frequency, classroom homework submission heatmaps (GitHub-fidelity contribution graph), code churn statistics, and team leaderboards.
5. **Rigorous Multi-Remote QA & Criterion Benchmarking (`phase-5`):** Multi-host mock integration test suite, 3-way merge conflict fuzz testing, and Criterion benchmarks ensuring sub-30ms 3-way diff parsing and zero UI latency.

---

## 2. Phase Index & Detailed Specifications Map

Sprint 2 is structured into 5 cohesive phases:

| Phase | Days | Specification Document | Primary Target Crates & Modules | Key Deliverables & Milestones |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 1** | Days 1–5 | [phase-1-multi-remote-mirroring.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/phase-1-multi-remote-mirroring.md) | `gitupload-core`, `gitupload-git`, `gitupload-security`, `gitupload-app` | Multi-remote configuration, parallel async push/fetch via Tokio join sets, per-remote credential resolver, and aggregate progress streaming. |
| **Phase 2** | Days 6–10 | [phase-2-interactive-3way-merge.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/phase-2-interactive-3way-merge.md) | `gitupload-git`, `gitupload-app`, `ui/` | `git2::MergeOptions` 3-way conflict calculator, Monaco diff integration, chunk resolver, and conflict-free commit builder. |
| **Phase 3** | Days 11–15 | [phase-3-native-ssh-agent.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/phase-3-native-ssh-agent.md) | `gitupload-security`, `gitupload-git`, `gitupload-github` | Pure-Rust ED25519/RSA key generator, in-memory pageant/OpenSSH named pipe bridge, and API public key uploader. |
| **Phase 4** | Days 16–20 | [phase-4-team-analytics-heatmap.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/phase-4-team-analytics-heatmap.md) | `gitupload-storage`, `gitupload-app`, `ui/` | SQLite relational aggregation queries, 365-day contribution heatmap, commit velocity calculations, and classroom leaderboard components. |
| **Phase 5** | Days 21–25 | [phase-5-sprint2-qa-verification.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-2/phase-5-sprint2-qa-verification.md) | All crates, `tests/`, `benches/` | Multi-remote integration harness, automated 3-way merge stress tests, Criterion performance benchmarks, and Clippy zero-warning audit. |

---

## 3. Crate Impact & Architectural Additions

Sprint 2 builds upon the 10-crate workspace without introducing breaking changes:

```
+----------------------------------------------------------------------------------------------------+
|                                    gitupload-app (Tauri v2 Desktop Shell)                          |
+----+--------+--------+--------+--------+--------+--------+--------+--------+-----------------------+
     |        |        |        |        |        |        |        |        |
     |        |        |        |        |        |        |        |        v
     |        |        |        |        |        |        |        +---> gitupload-watcher (Multi-remote watch)
     |        |        |        |        |        |        v
     |        |        |        |        |        +---> gitupload-ai (AI Conflict Resolution Assistant)
     |        |        |        |        v
     |        |        |        +---> gitupload-github (SSH Key Upload & Org Analytics)
     |        |        v
     |        +---> gitupload-ghcli (CLI mirror command integration)
     |        |
     |        +---> gitupload-hooks (Pre-push multi-remote verification)
     |        |        |
     |        v        v
     +---> gitupload-git (Multi-Remote Mirroring, 3-Way Merge Engine, DAG Analytics)
     |        |
     |        v
     +---> gitupload-security (Native In-Memory SSH Agent & Keyring Remote Mapping)
     |        |
     |        v
     +---> gitupload-storage (Analytics Schema, Heatmaps & Remote Registry Tables)
     |        |
     v        v
   gitupload-core (Remote Models, Conflict Types, SSH Key Models, Analytics DTOs)
```

---

## 4. Definition of Ready (DoR)

Before starting any task in Sprint 2:
1. **Sprint 1 Complete:** Sprint 1 Phase 1 through Phase 5 must be implemented, verified by `@verifier`, and passing `cargo check --workspace`.
2. **Schema Extensibility:** Storage migrations must increment from Version 1 without data loss in `app_settings` or `push_history`.
3. **Security Invariant:** Multi-remote credentials must never be written to `.git/config` as plaintext URLs (`https://token@host/`). All credentials must route through `gitupload-security::keyring`.

---

## 5. Definition of Done (DoD)

A task or phase in Sprint 2 is considered complete only when:
1. **Zero Warnings:** Pass `cargo check --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
2. **Test Coverage:** 100% of unit and integration test fixtures for the sprint pass (`cargo test -p <crate_name>`).
3. **Performance SLAs:** 
   - Multi-remote parallel push finishes within **1.2x of the slowest single remote**.
   - 3-Way merge conflict calculation executes in **< 30ms** for files up to 5,000 lines.
   - Idle memory footprint remains **< 28 MB Private Working Set RAM**.
4. **Safety & Zero-Unsafe:** Enforce `#![forbid(unsafe_code)]` on all new modules.
5. **Formal Audit:** Formal verification sign-off posted to the swarm message board by `@verifier`.
