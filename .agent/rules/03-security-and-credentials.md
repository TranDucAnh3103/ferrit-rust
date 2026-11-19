# Rule 03: Security, Secrets & Credential Management
**Scope:** Authentication, Tokens, Keyring, Memory Hygiene, Log Sanitization  
**Severity:** Critical (Zero-Tolerance Security Violations)  

---

## 1. Zero-Plaintext Storage Policy (SEC-01)

1. **Banned Files for Secrets**:
   - **NEVER** write GitHub Personal Access Tokens (PAT), OAuth bearer tokens, or AI API keys to plaintext `.env` files, `config.json`, or unencrypted SQLite columns.
   - Any commit introducing plaintext credentials will be automatically rejected.

2. **OS Credential Vault Requirement**:
   - All secrets MUST be stored exclusively in the native **OS Credential Vault** via the `keyring` crate.
   - On Windows, this maps directly to the **Windows Credential Manager** (Generic Credentials, DPAPI encrypted).
   - Target namespace format: `gitupload:github:account_<uuid>` or `gitupload:ai:provider_<name>`.

---

## 2. Memory Hygiene & In-Memory Secrets

1. **`secrecy::SecretString`**:
   - Tokens retrieved from the vault or received from the UI must immediately be wrapped in `secrecy::SecretString`.
   - Never print or debug-format secrets (`{:?}` prints `"[REDACTED]"` automatically).
2. **`zeroize::Zeroize` on Drop**:
   - Secret buffers must be zeroized upon drop to ensure they do not linger in memory pages or swap files.
3. **Restricted Exposure**:
   - Use `.expose_secret()` only at the exact point of HTTP request header creation (`Authorization: Bearer <secret>`) or `libgit2` credential callbacks.
   - Do not pass exposed secrets across thread boundaries or store raw strings in long-lived state structs.

---

## 3. Real-Time Token Log Sanitization (SEC-02)

1. **Interception Pipeline**:
   - All log messages emitted to disk, console, or Tauri IPC channels MUST pass through `gitupload_security::sanitizer::sanitize_log_message`.
2. **Regex Patterns**:
   - `https://[^@\s/]+@github\.com` ➔ `https://***@github.com`
   - `gh[pousr]_[A-Za-z0-9_]{36,255}` ➔ `[REDACTED_TOKEN]`
   - `github_pat_[A-Za-z0-9_]{82}` ➔ `[REDACTED_PAT]`
   - `Bearer [A-Za-z0-9\-._~+/]+=*` ➔ `Bearer [REDACTED]`

---

## 4. Binary Integrity & Sandboxing (`gitupload-ghcli`)

1. **Cryptographic Checksum Verification**:
   - Every external binary downloaded by `gitupload-ghcli` (e.g. `gh.exe` portable zip) MUST be verified against its official SHA256 checksum published in GitHub Releases.
   - If the SHA256 digest fails to match, the downloaded archive MUST be deleted immediately, and the process aborted.
2. **Zero-Admin Local Sandboxing**:
   - Downloaded binaries MUST be extracted into the user's local application data directory: `%APPDATA%/rust-gitupload/bin/gh.exe`.
   - NEVER request UAC elevation or write to `Program Files` or system directories.
3. **Process Execution Flags**:
   - When spawning sandboxed or system CLI tools, Windows creation flags MUST specify `CREATE_NO_WINDOW (0x08000000)` to prevent flashing black CMD windows.
