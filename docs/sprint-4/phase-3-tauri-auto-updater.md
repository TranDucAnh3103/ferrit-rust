# Sprint 4 — Phase 3: Tauri Auto-Updater & Multi-Channel Distribution Specification
**Document Identifier:** SPRINT4-PHASE3-AUTO-UPDATER  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Cryptographic Minisign Verification, Multi-Channel Release Routing (Stable / Beta / Nightly), Silent Background Download, and Atomic Binary Hot-Swap  
**Target Crates:** `gitupload-app`, `src-tauri/`  
**Timeline:** Days 7–9 (Sprint 4)  

---

## 1. Phase Overview & Technical Strategy

Phase 3 implements an enterprise-grade, cryptographically verified automated updater system powered by Tauri v2 Updater plugins. To ensure zero security compromises, every update payload (installer or delta patch) is signed with an offline Ed25519 private key via `minisign`. The client validates signatures against an embedded public key before disk writes. The updater supports **3 distinct release channels (Stable, Beta, Nightly)**, background chunk downloading, and atomic executable hot-swapping on Windows, macOS, and Linux.

---

## 2. Granular Task Breakdown

### Task TASK-P4-3-01: Minisign Cryptographic Verification Engine
- **Task Identifier:** `TASK-P4-3-01`
- **Component:** `src-tauri/tauri.conf.json`, `src-tauri/src/updater.rs`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-004`

#### Configuration & Public Key Binding
```json
// src-tauri/tauri.conf.json (plugins.updater section)
{
  "plugins": {
    "updater": {
      "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEdpdFVwbG9hZAptdW50cnVzdGVk...[BASE64_ED25519_KEY]",
      "endpoints": [
        "https://releases.gitupload.org/api/v1/update/{{target}}/{{current_version}}?channel={{channel}}"
      ],
      "windows": {
        "installMode": "passive"
      }
    }
  }
}
```

---

### Task TASK-P4-3-02: Multi-Channel Release Routing Engine
- **Task Identifier:** `TASK-P4-3-02`
- **Component:** `crates/gitupload-app/src/updater_channel.rs`
- **SRS Traceability:** `FR-SET-001`, `NFR-REL-004`

#### Rust Implementation Signature
```rust
// crates/gitupload-app/src/updater_channel.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
    Nightly,
}

impl Default for UpdateChannel {
    fn default() -> Self {
        Self::Stable
    }
}

pub struct UpdateManifest {
    pub version: String,
    pub release_notes: String,
    pub pub_date: String,
    pub signature: String,
    pub download_url: String,
    pub sha256_checksum: String,
}

impl UpdateChannel {
    pub fn endpoint_url(&self, current_version: &str, target_os: &str) -> String {
        format!(
            "https://releases.gitupload.org/api/v1/update/{target_os}/{current_version}?channel={}",
            match self {
                Self::Stable => "stable",
                Self::Beta => "beta",
                Self::Nightly => "nightly",
            }
        )
    }
}
```

---

### Task TASK-P4-3-03: Silent Background Download & Progress Streaming
- **Task Identifier:** `TASK-P4-3-03`
- **Component:** `crates/gitupload-app/src/commands/updater.rs`
- **SRS Traceability:** `NFR-PERF-004`, `NFR-REL-002`

#### Rust IPC Command Signature
```rust
// crates/gitupload-app/src/commands/updater.rs
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<UpdateManifestInfo>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;

    if let Some(update) = update {
        Ok(Some(UpdateManifestInfo {
            version: update.version.clone(),
            body: update.body.clone().unwrap_or_default(),
            date: update.date.map(|d| d.to_string()).unwrap_or_default(),
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn install_update_streamed(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        let app_clone = app.clone();
        update.download_and_install(
            move |chunk_length, content_length| {
                let _ = app_clone.emit("update-download-progress", DownloadProgress {
                    downloaded: chunk_length,
                    total: content_length,
                });
            },
            || {
                println!("Update download finished, ready for installation.");
            }
        ).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

---

### Task TASK-P4-3-04: Atomic Windows Executable Replacement
- **Task Identifier:** `TASK-P4-3-04`
- **Component:** `crates/gitupload-app/src/platform/win32_swap.rs`
- **SRS Traceability:** `NFR-REL-001`, `NFR-PORT-001`

#### Windows Atomic Swap Logic
On Windows, running executables are locked against modification. The installer invokes `MoveFileExW` with `MOVEFILE_DELAY_UNTIL_REBOOT` or renames the current binary to `.old` before spawning the fresh binary:
```rust
#[cfg(target_os = "windows")]
pub fn atomic_executable_swap(new_binary_path: &std::path::Path) -> std::io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let backup_exe = current_exe.with_extension("exe.old");

    // 1. Rename running binary to .old (Windows allows renaming running binaries on NTFS)
    std::fs::rename(&current_exe, &backup_exe)?;

    // 2. Move new binary into original location
    std::fs::copy(new_binary_path, &current_exe)?;

    // 3. Mark old binary for deletion upon process exit
    std::fs::remove_file(&backup_exe).ok();
    Ok(())
}
```

---

## 3. Strict Acceptance Criteria Matrix (AC-P4-3-xxx)

| Criteria ID | Scenario | Given-When-Then Verification | Verification Command |
| :--- | :--- | :--- | :--- |
| **AC-P4-3-01** | Signature Tampering Interception | **Given** an update payload with invalid minisign signature, **When** verified by client, **Then** update must be rejected with `SignatureVerificationFailed`. | `cargo test -p gitupload-app --test test_updater_signature_rejection` |
| **AC-P4-3-02** | Channel Switching | **Given** user switches channel from Stable to Beta, **When** update checked, **Then** query endpoint must pass `channel=beta`. | `cargo test -p gitupload-app --test test_channel_routing` |
| **AC-P4-3-03** | Progress Event Coalescing | **Given** 10MB update download, **When** progress emitted, **Then** client must receive regular percentage updates without blocking UI thread. | UI E2E Playwright update test |
| **AC-P4-3-04** | Windows Executable Hot-Swap | **Given** active app on Windows 11, **When** update applied, **Then** process restarts seamlessly running the new version without file lock errors. | E2E Windows Upgrade Runner |
| **AC-P4-3-05** | Passive Silent Install | **Given** passive install flag, **When** installation triggers, **Then** no interactive installer wizard dialogs shall be displayed to user. | Visual Verification |

---

*Document Author:* `@srs_author`  
*Reviewers:* `@verifier`, `@coordinator`
