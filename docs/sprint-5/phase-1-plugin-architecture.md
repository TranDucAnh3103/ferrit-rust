# Sprint 5 Phase 1 — Sandboxed WebAssembly Plugin Architecture
**Document Identifier:** SPRINT5-PHASE1-SPEC  
**Target Milestone:** Release v2.6.0-extensible  
**Status:** Approved Technical Architecture  
**Parent Sprint Guide:** [Sprint 5 Master Guide](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md)  

---

## 1. Overview & Architectural Objectives

Phase 1 establishes the core extension runtime for **GitUpload Desktop** using **WebAssembly (WASM)**. Third-party developers and enterprise teams can extend GitUpload's behavior—such as custom commit message validation, branch naming enforcement, AI prompt sanitization, and specialized diff viewers—without native code execution risks.

### 1.1 Architectural Pillars
1. **Memory Safety & Sandbox Isolation**: Plugins run inside isolated WebAssembly instances managed by `wasmtime`. Each plugin is constrained to a strict **32 MB linear memory ceiling**.
2. **Deterministic Execution Limits**: Instruction metering via `wasmtime` fuel prevents infinite loops or CPU exhaustion (`store.set_fuel(10_000_000)`).
3. **Capability-Based Permissions**: Plugins have **zero** access to the host filesystem, environment variables, or sockets unless explicitly declared in `plugin.toml` and authorized by the user.
4. **Resilient Host Protection**: If a guest plugin triggers a panic, out-of-bounds memory access, or fuel exhaustion, the host traps the error, records a diagnostic event, and cleanly continues desktop operations with zero downtime.

```
+------------------------------------------------------------------------------------------+
|                              WASM Plugin Sandbox Architecture                            |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   | GitUpload Host Application (Tauri v2 + Tokio)                                    |   |
|   |                                                                                  |   |
|   |  +------------------------+      +------------------------+                      |   |
|   |  | PluginLifecycleManager | <--> | Host Function Linker   |                      |   |
|   |  | (Discovery / Stores)   |      | (git_read, host_log)   |                      |   |
|   |  +-----------+------------+      +-----------+------------+                      |   |
|   +--------------|-------------------------------|-----------------------------------+   |
|                  | Instantiates                  | Host Function Calls                   |
|                  v                               v                                       |
|   +----------------------------------------------------------------------------------+   |
|   | Wasmtime Sandboxed Guest Instance                                                |   |
|   |                                                                                  |   |
|   |  +-----------------------------+  +-------------------------------------------+  |   |
|   |  | Linear Memory (Max: 32 MB)  |  | Fuel Metering Budget (10M Units)          |  |   |
|   |  +-----------------------------+  +-------------------------------------------+  |   |
|   |  | Guest Entrypoints:                                                         |  |   |
|   |  | - plugin_init() -> i32                                                     |  |   |
|   |  | - plugin_on_commit_msg(ptr, len) -> i32                                    |  |   |
|   |  | - plugin_on_ai_prompt(ptr, len) -> i32                                     |  |   |
|   |  +----------------------------------------------------------------------------+  |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
```

---

## 2. Plugin Manifest Schema (`plugin.toml`)

Every plugin package must include a root `plugin.toml` manifest defining identity, permissions, and entrypoint binary:

```toml
[plugin]
id = "org.gitupload.plugins.jira-linter"
name = "Jira Ticket Commit Linter"
version = "1.0.4"
author = "DevOps Engineering Team <devops@enterprise.internal>"
description = "Enforces Jira issue ticket prefix (e.g. PROJ-1234) on all Git commit messages."
entrypoint = "plugin.wasm"
homepage = "https://github.com/gitupload/plugins-jira"

[capabilities]
git_read = true          # Read branch name, status, staged files
git_write = false        # Disallowed from mutating git state
commit_hook = true       # Intercept commit-msg and pre-commit
ai_prompt_filter = false # Not intercepting AI generation
network_access = false   # No external HTTP calls permitted
max_memory_mb = 16       # Plugin declares lower memory profile
```

---

## 3. Host-Guest ABI & Rust Implementation

### 3.1 Host State & Resource Limiting

```rust
// crates/gitupload-plugins/src/sandbox.rs
use std::sync::Arc;
use thiserror::Error;
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Module, ResourceLimiter, Store, StoreLimits,
    StoreLimitsBuilder, Trap,
};

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin manifest error: {0}")]
    ManifestError(String),
    #[error("WASM engine error: {0}")]
    WasmEngineError(#[from] wasmtime::Error),
    #[error("Plugin execution trapped: {0}")]
    ExecutionTrapped(String),
    #[error("Permission denied: capability '{0}' not granted")]
    PermissionDenied(String),
    #[error("Linear memory exceeded 32MB ceiling")]
    MemoryCeilingExceeded,
    #[error("Fuel exhausted: execution exceeded instruction budget")]
    FuelExhausted,
}

pub struct HostState {
    pub limits: StoreLimits,
    pub plugin_id: String,
    pub capabilities: PluginCapabilities,
    pub staged_files: Vec<String>,
    pub current_branch: String,
}

impl ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, wasmtime::ResourceLimiterError> {
        self.limits.memory_growing(current, desired, maximum)
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> Result<bool, wasmtime::ResourceLimiterError> {
        self.limits.table_growing(current, desired, maximum)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PluginCapabilities {
    pub git_read: bool,
    pub git_write: bool,
    pub commit_hook: bool,
    pub ai_prompt_filter: bool,
    pub network_access: bool,
    pub max_memory_mb: u32,
}
```

### 3.2 Engine Setup & Fuel Limiting

```rust
// crates/gitupload-plugins/src/engine.rs
use super::sandbox::*;
use wasmtime::{Config, Engine, Linker, StoreLimitsBuilder};

pub struct PluginEngine {
    engine: Engine,
}

impl PluginEngine {
    pub fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        // Enable instruction fuel metering to terminate infinite loops
        config.consume_fuel(true);
        // Epoch interruption for background async cancellation
        config.epoch_interruption(true);
        // Strict memory sandboxing
        config.wasm_memory64(false);

        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    pub fn create_store(&self, state: HostState) -> wasmtime::Store<HostState> {
        let max_bytes = (state.capabilities.max_memory_mb.min(32) as usize) * 1024 * 1024;
        let limits = StoreLimitsBuilder::new()
            .memory_size(max_bytes)
            .instances(1)
            .tables(10)
            .build();

        let mut store_state = state;
        store_state.limits = limits;

        let mut store = wasmtime::Store::new(&self.engine, store_state);
        store.limiter(|s| s);
        // Assign 10,000,000 fuel units per execution
        store.set_fuel(10_000_000).expect("Fuel must be set");
        store
    }
}
```

### 3.3 Host Functions Linker

```rust
// crates/gitupload-plugins/src/linker.rs
use super::sandbox::*;
use wasmtime::{Caller, Linker};

pub fn build_linker(engine: &wasmtime::Engine) -> Result<Linker<HostState>, PluginError> {
    let mut linker = Linker::new(engine);

    // host_log(level: u32, ptr: u32, len: u32)
    linker.func_wrap(
        "env",
        "host_log",
        |mut caller: Caller<'_, HostState>, level: u32, ptr: u32, len: u32| -> Result<(), wasmtime::Trap> {
            let memory = caller
                .get_export("memory")
                .and_then(|m| m.into_memory())
                .ok_or_else(|| wasmtime::Trap::MemoryOutOfBounds)?;

            let data = memory.data(&caller);
            let slice = data
                .get(ptr as usize..(ptr + len) as usize)
                .ok_or_else(|| wasmtime::Trap::MemoryOutOfBounds)?;

            let msg = String::from_utf8_lossy(slice);
            let plugin_id = &caller.data().plugin_id;
            tracing::info!(target: "gitupload::plugins", "[Plugin: {}][Level {}] {}", plugin_id, level, msg);
            Ok(())
        },
    )?;

    // host_get_branch(out_ptr: u32, max_len: u32) -> u32
    linker.func_wrap(
        "env",
        "host_get_branch",
        |mut caller: Caller<'_, HostState>, out_ptr: u32, max_len: u32| -> Result<u32, wasmtime::Trap> {
            if !caller.data().capabilities.git_read {
                return Err(wasmtime::Trap::NoFuel); // Unauthorized capability
            }

            let branch_bytes = caller.data().current_branch.as_bytes();
            let copy_len = branch_bytes.len().min(max_len as usize);

            let memory = caller
                .get_export("memory")
                .and_then(|m| m.into_memory())
                .ok_or_else(|| wasmtime::Trap::MemoryOutOfBounds)?;

            let data = memory.data_mut(&mut caller);
            data[out_ptr as usize..out_ptr as usize + copy_len].copy_from_slice(&branch_bytes[..copy_len]);
            Ok(copy_len as u32)
        },
    )?;

    Ok(linker)
}
```

---

## 4. Work Breakdown & Granular Tasks

### TASK-P5-1-01: Workspace Crate Scaffolding & Wasmtime Engine Setup
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/Cargo.toml`  
  `rust-gitupload/crates/gitupload-plugins/src/lib.rs`  
  `rust-gitupload/crates/gitupload-plugins/src/engine.rs`
- **Actions:**
  - Initialize `gitupload-plugins` crate with `#![forbid(unsafe_code)]`.
  - Add dependencies: `wasmtime = "26.0"`, `thiserror = "2.0"`, `serde = { version = "1.0", features = ["derive"] }`, `toml = "0.8"`, `tracing = "0.1"`.
  - Configure `PluginEngine` with fuel metering enabled (`consume_fuel(true)`) and epoch interruption.

### TASK-P5-1-02: Manifest Parsing & Permission Model
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/src/manifest.rs`
- **Actions:**
  - Define `PluginManifest` and `PluginCapabilities` structs with strict validation.
  - Implement reverse-DNS identifier validator (`^[a-zA-Z0-9_-]+(\.[a-zA-Z0-9_-]+)+$`).
  - Restrict `max_memory_mb` to a hard upper bound of 32 MB.

### TASK-P5-1-03: Host Linker & Capability Gateway
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/src/linker.rs`  
  `rust-gitupload/crates/gitupload-plugins/src/abi.rs`
- **Actions:**
  - Implement `host_log`, `host_get_branch`, `host_get_staged_files`, and `host_read_file_diff`.
  - Enforce explicit boolean checks against `caller.data().capabilities.<flag>` before servicing any host function.
  - Return descriptive traps if unpermitted APIs are invoked.

### TASK-P5-1-04: Hook Interception Engine
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/src/runner.rs`  
  `rust-gitupload/crates/gitupload-hooks/src/runner.rs`
- **Actions:**
  - Integrate `gitupload-plugins` into the native git hooks lifecycle (`commit-msg`, `pre-commit`).
  - Pass commit messages across linear memory into the guest `plugin_on_commit_msg` entrypoint.
  - Interpret return codes: `0` indicates success, non-zero indicates rejection with human-readable error payload extracted from guest linear memory.

### TASK-P5-1-05: Tauri IPC Bridge & Plugin Store
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/commands/plugins.rs`  
  `rust-gitupload/crates/gitupload-app/src/state.rs`
- **Actions:**
  - Expose IPC commands: `plugin_list_installed`, `plugin_toggle_active`, `plugin_inspect_manifest`.
  - Store plugin enabled/disabled states in SQLite `user_plugins` table.

---

## 5. Acceptance Criteria (Given-When-Then)

### AC-P5-1-01: Sandboxed Execution & 32MB Memory Ceiling
- **Given:** A third-party WASM plugin running in `gitupload-plugins`.
- **When:** The guest attempts to allocate more than 32 MB of linear memory.
- **Then:** The `ResourceLimiter` traps the allocation request, returns an out-of-memory error to the guest, and ensures host heap usage remains unaffected.

### AC-P5-1-02: Fuel Metering & Infinite Loop Termination
- **Given:** A malicious or buggy plugin containing an infinite `loop {}`.
- **When:** The host invokes `plugin_on_commit_msg()` with 10,000,000 fuel units allocated.
- **Then:** The `wasmtime` engine exhausts the fuel counter within 5ms, halts guest execution with a `Trap::NoFuel`, and returns `PluginError::FuelExhausted` to the host without freezing the UI thread.

### AC-P5-1-03: Capability Permission Enforcement
- **Given:** A plugin manifest with `capabilities.git_read = false`.
- **When:** The plugin guest function invokes the exported `host_get_branch()` host API.
- **Then:** The host immediately refuses execution, triggers a permission trap, and writes a warning log with the offending plugin ID.

### AC-P5-1-04: Git Commit Hook Interception via WASM
- **Given:** The `jira-linter` plugin installed with `commit_hook = true`.
- **When:** A user enters commit message `"fix: typo in readme"` without a Jira ticket key.
- **Then:** The WASM guest returns status code `1` with message `"Commit message must start with [PROJ-XXXX]"`, causing GitUpload to abort the commit and display the rejection in Primer UI.

### AC-P5-1-05: Tauri IPC Plugin Management & Crash Isolation
- **Given:** An active plugin that abruptly panics or crashes inside its WASM module.
- **When:** A Git operation triggers the faulted plugin.
- **Then:** Tauri IPC receives a structured error payload, the desktop UI displays a non-blocking notification, and GitUpload continues operating normally.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.6.0-extensible
