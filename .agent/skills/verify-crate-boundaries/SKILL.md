---
name: verify-crate-boundaries
description: Inspects Cargo workspace manifests and source files across all 10 crates to enforce strict architectural layering, prevent circular/horizontal coupling, and ensure unsafe-code bans.
---

# Skill: verify-crate-boundaries

## Overview
This skill audits the 10 modular workspace crates of `rust-gitupload` to ensure strict conformance with the C4 Level 3 Component architecture defined in [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md) and Rule [01-architecture-principles.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/.agent/rules/01-architecture-principles.md).

## Execution Steps

### 1. Inspect Workspace Member Manifests
Check that all 10 member crates are declared in the root `Cargo.toml`:
- `crates/gitupload-core`
- `crates/gitupload-security`
- `crates/gitupload-storage`
- `crates/gitupload-git`
- `crates/gitupload-hooks`
- `crates/gitupload-ghcli`
- `crates/gitupload-github`
- `crates/gitupload-ai`
- `crates/gitupload-watcher`
- `crates/gitupload-app`

### 2. Verify Hierarchical Layering Rules
Execute an inspection of `Cargo.toml` in each crate directory:
1. **`gitupload-core` Isolation**:
   - MUST NOT depend on any other crate in `crates/`.
2. **Horizontal Decoupling**:
   - `gitupload-github` MUST NOT depend on `gitupload-git` or `gitupload-ai`.
   - `gitupload-ai` MUST NOT depend on `gitupload-github` or `gitupload-git`.
   - `gitupload-ghcli` MUST NOT depend on `gitupload-storage` or `gitupload-github`.
   - `gitupload-storage` MUST NOT depend on `gitupload-git` or `gitupload-github`.
3. **Application Shell Integration**:
   - Only `gitupload-app` is permitted to depend on all domain crates simultaneously to expose them over Tauri IPC.

### 3. Verify `#![forbid(unsafe_code)]` Policy
Inspect the root of every library crate:
- Check that `crates/gitupload-*/src/lib.rs` contains `#![forbid(unsafe_code)]`.
- The only permissible exceptions are strictly commented Win32 FFI blocks in `gitupload-app` (Single Instance Named Mutex) and `gitupload-git::cli` (Windows Job Objects).

### 4. Verify Zero Plaintext Secrets
Scan the repository files for forbidden plaintext token strings:
- Search for any hardcoded strings matching `ghp_`, `github_pat_`, `sk-ant-`, `sk-proj-`, or `AIzaSy`.
- Confirm that `.env` is listed in `.gitignore`.

### 5. Validate Compilation & Tree Soundness
Run Cargo metadata and tree checks:
```powershell
cargo check --workspace
cargo tree --workspace --duplicates
```
Ensure there are no conflicting version duplicates or circular dependency warnings.
