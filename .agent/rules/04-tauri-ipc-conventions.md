# Rule 04: Tauri v2 IPC & Event Streaming Conventions
**Scope:** IPC Commands, Event Emitters, Frontend-Backend Contract  
**Severity:** High  

---

## 1. Tauri v2 Command Conventions

1. **Command Naming Standard**:
   - Commands MUST follow the `<domain>_<action>` snake_case convention:
     - `auth_save_token`, `auth_list_accounts`, `auth_sync_from_gh`
     - `git_inspect_repo`, `git_start_push`, `git_repush_repo`, `git_start_clone`
     - `graph_get_commits`, `graph_cherry_pick`, `graph_revert`, `graph_reset`
     - `ghcli_check_status`, `ghcli_install_silent`
     - `pr_list`, `pr_create`, `pr_merge`
     - `hooks_install`, `hooks_validate_commit`
     - `ai_generate_commit`, `ai_generate_pr_description`, `ai_chat_send`

2. **Command Signatures**:
   - All commands performing I/O or background processing must be `async`.
   - Commands must return `Result<T, IpcErrorPayload>`.
   - Managed state is injected using `tauri::State<'_, AppState>`.

3. **Standardized Error Envelope**:
   ```rust
   #[derive(serde::Serialize)]
   pub struct IpcErrorPayload {
       pub code: String,       // e.g. "GIT_AUTH_FAILED", "ORGANIZATION_FORBIDDEN"
       pub message: String,    // User-friendly descriptive message
       pub recoverable: bool,  // Whether the frontend should show a retry button
   }
   ```

---

## 2. Event Streaming Channels

For streaming long-running progress and high-volume logs, use Tauri events instead of polling:

| Channel Name | Payload Struct | Frequency / Debounce |
| :--- | :--- | :--- |
| `job-progress` | `JobProgressPayload` | Debounced to 30ms (max 33 updates/sec) |
| `job-log-batch` | `JobLogBatchPayload` | Micro-batched at 60Hz (max 60 updates/sec) |
| `ai-stream-chunk` | `AiStreamChunkPayload` | Streamed per SSE chunk received |
| `watcher-event` | `WatcherAlertPayload` | Debounced to 500ms after last file write |

### 2.1 Micro-Batching Implementation Rule
- **NEVER** call `app.emit("log", line)` on every single line of stdout/stderr in a tight loop.
- Doing so saturates the Webview IPC message channel, drops frame rates, and causes UI stutter.
- Always accumulate lines into a bounded buffer and flush periodically (every 16–30ms) or when the batch exceeds 50 lines.

---

## 3. Tauri v2 Security & Capabilities

1. **Explicit Capability Files**:
   - Commands must be explicitly registered under `src-tauri/capabilities/default.json`.
   - Banned permissions: Do not grant wildcard shell or unrestricted filesystem access.
2. **Path Normalization**:
   - All filesystem paths passed across IPC must be canonicalized and checked against path traversal attacks (`..`).
