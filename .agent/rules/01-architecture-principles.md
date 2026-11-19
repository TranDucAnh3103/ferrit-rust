# Rule 01: Architecture Principles & Modular Boundaries
**Scope:** Architecture Design, Crate Organization, Dependency Management  
**Severity:** Critical  

---

## 1. C4 Architecture & Crate Boundaries

All code in `rust-gitupload` must strictly adhere to the C4 Level 3 Component model defined in [02_ARCHITECTURE.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/02_ARCHITECTURE.md). The 10 workspace crates have explicit dependency hierarchies that MUST NOT be violated.

### 1.1 Dependency Graph Rules

```
gitupload-core (No workspace dependencies)
      ▲
      │
gitupload-security (Depends on core)
      ▲
      │
gitupload-storage, gitupload-git, gitupload-hooks, gitupload-ghcli, gitupload-github, gitupload-ai, gitupload-watcher
(May depend on core and security; NEVER depend on each other horizontally)
      ▲
      │
gitupload-app (The Tauri v2 binary shell: depends on all library crates)
```

1. **`gitupload-core` is the Root Anchor**:
   - Contains only shared domain models, error hierarchies, traits, and cancellation tokens.
   - It MUST NOT depend on any other crate in the workspace.
2. **Zero Circular Dependencies**:
   - Circular crate dependencies are strictly forbidden by Cargo.
3. **Horizontal Isolation**:
   - `gitupload-github` MUST NOT depend on `gitupload-git`.
   - `gitupload-ai` MUST NOT depend on `gitupload-github`.
   - `gitupload-ghcli` MUST NOT depend on `gitupload-storage`.
   - Any cross-domain coordination must take place in `gitupload-app` command handlers or coordinators.

---

## 2. Concurrency & Non-Blocking Main Loop

1. **UI Thread Protection**:
   - The Tauri UI thread must NEVER block on synchronous I/O or CPU operations.
   - Synchronous Git calls (`git2::Repository::open`, tree walking, diff computation) MUST be dispatched to `tokio::task::spawn_blocking`.
2. **Actor-Based Worker Channels**:
   - Git and network jobs run as isolated Tokio tasks.
   - Long-running jobs stream progress updates and log lines through `tokio::sync::mpsc` channels.
3. **Throttled Event Streaming**:
   - High-throughput logging must be buffered and emitted to the Webview at a maximum rate of 60Hz (30ms micro-batching window) to prevent Webview render thread starvation.
4. **Cooperative Cancellation**:
   - Every background task must accept a `tokio_util::sync::CancellationToken` and check `token.is_cancelled()` at safe checkpoints.

---

## 3. Dual-Engine Git Strategy

1. **`libgit2` as Primary In-Process Engine**:
   - Used for `init`, `status`, `add`, `commit`, `diff`, `revwalk`, branch CRUD, and HTTPS push/clones with in-memory tokens.
   - Guarantees < 3ms execution time and zero terminal flashes on Windows.
2. **CLI Fallback with Windows Job Objects**:
   - Used when specialized transports (GCM, complex SSH, or Git LFS) are needed.
   - Every CLI process must be spawned inside a Win32 Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` to ensure child processes (`git-remote-https.exe`, `ssh.exe`) are terminated atomically when cancelled.

---

## 4. Pure Cloud AI Policy

1. **Zero Local Model Downloads**:
   - The desktop client MUST NOT download or run local LLMs (Ollama, Llama-cpp, Candle).
2. **Cloud AI Multi-Provider Engine**:
   - Strictly leverage Cloud AI APIs (Google Gemini 2.0/1.5, OpenAI GPT-4o, Claude 3.5 Sonnet, Groq, OpenRouter) via streaming SSE (`eventsource-stream`).
   - Guarantees an ultra-lean idle memory footprint (< 28 MB RAM).
