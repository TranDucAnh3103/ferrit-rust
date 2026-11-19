# Sprint 5 Phase 4 — Decentralized Community Plugin Marketplace
**Document Identifier:** SPRINT5-PHASE4-SPEC  
**Target Milestone:** Release v2.6.0-extensible  
**Status:** Approved Technical Architecture  
**Parent Sprint Guide:** [Sprint 5 Master Guide](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md)  

---

## 1. Overview & Architectural Objectives

Phase 4 introduces the **Decentralized Community Plugin Marketplace** to **GitUpload Desktop**. This subsystem enables developers to discover, install, update, and manage verified WebAssembly extensions directly from the GitHub Primer UI.

### 1.1 Architectural Pillars
1. **Decentralized HTTPS Registry**: Global CDN-cached catalog index (`https://marketplace.gitupload.org/v1/plugins.json`) with automatic offline disk caching.
2. **Cryptographic Supply-Chain Security**: Mandatory **SHA256 payload integrity** and **Ed25519 publisher signature** verification before any plugin is unpacked.
3. **Atomic Installation & Rollback**: New plugins and updates are staged in temporary isolation (`.staging_<id>`) and atomically swapped only after passing verification.
4. **Explicit Capability Consent**: Transparent Primer UI permission modal that displays requested capabilities (Git Read, Commit Hooks, UI Tabs) before installation proceeds.

```
+------------------------------------------------------------------------------------------+
|                             Marketplace Installation Flow                                |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   | Primer UI Marketplace Explorer (Search, Filter, Verified Badges)                 |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | User Clicks "Install"                     |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Capability Review Modal (Explicit Confirmation of Permissions)                   |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | User Confirms                             |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Secure Downloader & Verification Pipeline                                        |   |
|   |                                                                                  |   |
|   |  1. Download bundle to .staging/                                                 |   |
|   |  2. Verify SHA256 checksum against index digest                                  |   |
|   |  3. Verify Ed25519 publisher signature via trusted Root CA / Publisher Key      |   |
|   |  4. Validate plugin.toml manifest and WASM binary structure                      |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Verification Passes                       |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Atomic Directory Rename into %APPDATA%\GitUpload\plugins\<plugin_id>\            |   |
|   | -> Registered in SQLite `user_plugins` & Hot-Loaded into Wasmtime Engine        |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
```

---

## 2. Marketplace Index Schema & Data Models

```rust
// crates/gitupload-core/src/marketplace.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplaceIndex {
    pub version: u32,
    pub updated_at: DateTime<Utc>,
    pub plugins: Vec<MarketplacePluginEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplacePluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: PluginCategory,
    pub download_url: String,
    pub sha256: String,
    pub ed25519_signature: String,
    pub publisher_pubkey: String,
    pub min_app_version: String,
    pub capabilities: Vec<String>,
    pub verified: bool,
    pub downloads_count: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Linter,
    AiPrompt,
    UiTheme,
    Webhook,
    Automation,
}
```

---

## 3. Cryptographic Verification & Atomic Installer

### 3.1 Ed25519 Signature & SHA256 Integrity Verification

```rust
// crates/gitupload-plugins/src/verify.rs
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerificationError {
    #[error("Checksum mismatch: expected {expected}, calculated {calculated}")]
    ChecksumMismatch { expected: String, calculated: String },
    #[error("Invalid Ed25519 signature format")]
    InvalidSignatureFormat,
    #[error("Ed25519 signature verification failed: signature does not match publisher public key")]
    SignatureVerificationFailed,
    #[error("Untrusted publisher public key")]
    UntrustedPublisher,
}

pub fn verify_plugin_bundle(
    bundle_bytes: &[u8],
    expected_sha256: &str,
    signature_hex: &str,
    publisher_pubkey_hex: &str,
) -> Result<(), VerificationError> {
    // 1. Verify SHA256 Checksum
    let mut hasher = Sha256::new();
    hasher.update(bundle_bytes);
    let calculated_sha256 = hex::encode(hasher.finalize());

    if !calculated_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(VerificationError::ChecksumMismatch {
            expected: expected_sha256.to_string(),
            calculated: calculated_sha256,
        });
    }

    // 2. Decode Public Key & Signature
    let pubkey_bytes = hex::decode(publisher_pubkey_hex)
        .map_err(|_| VerificationError::UntrustedPublisher)?;
    let verifying_key = VerifyingKey::from_bytes(
        pubkey_bytes
            .as_slice()
            .try_into()
            .map_err(|_| VerificationError::UntrustedPublisher)?,
    )
    .map_err(|_| VerificationError::UntrustedPublisher)?;

    let sig_bytes = hex::decode(signature_hex)
        .map_err(|_| VerificationError::InvalidSignatureFormat)?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|_| VerificationError::InvalidSignatureFormat)?;

    // 3. Cryptographic Verification
    verifying_key
        .verify_strict(bundle_bytes, &signature)
        .map_err(|_| VerificationError::SignatureVerificationFailed)?;

    Ok(())
}
```

### 3.2 Atomic Staging & Rollback Manager

```rust
// crates/gitupload-plugins/src/installer.rs
use super::verify::*;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PluginInstaller {
    plugins_dir: PathBuf,
}

impl PluginInstaller {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self { plugins_dir }
    }

    pub async fn install(
        &self,
        plugin_id: &str,
        bundle_bytes: &[u8],
        sha256: &str,
        signature: &str,
        pubkey: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Step 1: Verify cryptographic integrity
        verify_plugin_bundle(bundle_bytes, sha256, signature, pubkey)?;

        let target_dir = self.plugins_dir.join(plugin_id);
        let staging_dir = self.plugins_dir.join(format!(".staging_{}", plugin_id));

        if staging_dir.exists() {
            fs::remove_dir_all(&staging_dir)?;
        }
        fs::create_dir_all(&staging_dir)?;

        // Step 2: Unpack tar.gz archive into staging directory
        let tar = flate2::read::GzDecoder::new(bundle_bytes);
        let mut archive = tar::Archive::new(tar);
        archive.unpack(&staging_dir)?;

        // Step 3: Verify manifest exists and is valid
        let manifest_path = staging_dir.join("plugin.toml");
        if !manifest_path.exists() {
            fs::remove_dir_all(&staging_dir)?;
            return Err("Extracted package missing plugin.toml".into());
        }

        // Step 4: Atomic Swap
        if target_dir.exists() {
            let backup_dir = self.plugins_dir.join(format!(".backup_{}", plugin_id));
            if backup_dir.exists() {
                fs::remove_dir_all(&backup_dir)?;
            }
            fs::rename(&target_dir, &backup_dir)?;
            if let Err(e) = fs::rename(&staging_dir, &target_dir) {
                // Rollback from backup
                let _ = fs::rename(&backup_dir, &target_dir);
                return Err(format!("Atomic swap failed, rolled back: {}", e).into());
            }
            let _ = fs::remove_dir_all(&backup_dir);
        } else {
            fs::rename(&staging_dir, &target_dir)?;
        }

        Ok(target_dir)
    }

    pub fn uninstall(&self, plugin_id: &str) -> std::io::Result<()> {
        let target_dir = self.plugins_dir.join(plugin_id);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }
        Ok(())
    }
}
```

---

## 4. Primer UI Permission Review Modal

When installing a plugin requiring capabilities, the UI presents an explicit permission elevation dialog:

```
+--------------------------------------------------------------------+
|  Install "Jira Ticket Commit Linter" by DevOps Team               |
+--------------------------------------------------------------------+
|  This plugin requires the following capabilities:                  |
|                                                                    |
|  [x] Read Git Repository State                                     |
|      Can read current branch name and list of staged files.        |
|                                                                    |
|  [x] Intercept Commit Lifecycle (Hooks)                           |
|      Can inspect, validate, or reject commit messages.             |
|                                                                    |
|  [ ] Network Access (Denied - Plugin has no network permissions)   |
|  [ ] File System Access (Denied - Isolated inside WASM Sandbox)    |
|                                                                    |
|  Memory Allocation: Max 16 MB Linear Heap                          |
|  Author Signature: Verified (Ed25519 Key ID: 7a8f...3d2c)          |
+--------------------------------------------------------------------+
|  [ Cancel ]                                 [ Install & Enable ]   |
+--------------------------------------------------------------------+
```

---

## 5. Work Breakdown & Granular Tasks

### TASK-P5-4-01: Marketplace Index Schema & Catalog Client
- **Target Files:**  
  `rust-gitupload/crates/gitupload-core/src/marketplace.rs`  
  `rust-gitupload/crates/gitupload-app/src/marketplace/catalog.rs`
- **Actions:**
  - Define `MarketplaceIndex`, `MarketplacePluginEntry`, and `PluginCategory`.
  - Implement HTTPS fetcher querying `https://marketplace.gitupload.org/v1/plugins.json` with 10s timeout.
  - Implement disk caching at `%APPDATA%\GitUpload\cache\marketplace.json` for offline browsing.

### TASK-P5-4-02: Cryptographic Verification Engine
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/src/verify.rs`
- **Actions:**
  - Implement SHA256 checksum validator using `sha2`.
  - Implement Ed25519 signature validator using `ed25519-dalek`.
  - Embed GitUpload Official Root Public Key for verifying official certified plugins.

### TASK-P5-4-03: Atomic Downloader & Directory Swapper
- **Target Files:**  
  `rust-gitupload/crates/gitupload-plugins/src/installer.rs`
- **Actions:**
  - Download archive into temporary `.staging_<id>` directory.
  - Verify manifest schema and entrypoint existence.
  - Execute atomic directory rename, preserving `.backup_<id>` during replacement.
  - Automatically rollback if swapping encounters an I/O error.

### TASK-P5-4-04: Tauri IPC Bridge for Marketplace
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/commands/marketplace.rs`
- **Actions:**
  - Expose IPC commands: `marketplace_fetch_catalog`, `marketplace_install_plugin`, `marketplace_uninstall_plugin`.
  - Emit progress events (`marketplace:install-progress`) for large downloads.

### TASK-P5-4-05: Primer UI Marketplace & Consent Modal
- **Target Files:**  
  `rust-gitupload/ui/src/views/MarketplaceView.tsx`  
  `rust-gitupload/ui/src/components/marketplace/PluginCard.tsx`  
  `rust-gitupload/ui/src/components/marketplace/PermissionModal.tsx`
- **Actions:**
  - Build searchable, category-filtered grid of plugin cards in GitHub Primer design.
  - Implement capability consent modal prior to install dispatch.
  - Display verified badge with tooltip linking to publisher public key.

---

## 6. Acceptance Criteria (Given-When-Then)

### AC-P5-4-01: Catalog Discovery & Offline Cache Fallback
- **Given:** GitUpload Desktop launched without an active internet connection.
- **When:** The user navigates to the Marketplace tab.
- **Then:** The client loads cached catalog metadata from `%APPDATA%\GitUpload\cache\marketplace.json` and displays an "Offline Catalog" badge without UI freezing.

### AC-P5-4-02: SHA256 Integrity Verification on Download
- **Given:** A corrupted plugin download where a byte in the `.tar.gz` archive is altered in transit.
- **When:** The installer calculates the payload SHA256 digest.
- **Then:** The digest mismatch is detected, the temporary download is immediately erased, and an error is returned before any file is extracted.

### AC-P5-4-03: Ed25519 Publisher Signature Validation
- **Given:** A third-party plugin bundle signed with an unrecognized or forged private key.
- **When:** `verify_plugin_bundle()` executes.
- **Then:** Signature verification returns `VerificationError::SignatureVerificationFailed`, halting installation and preventing untrusted execution.

### AC-P5-4-04: Atomic Installation with Rollback on Failure
- **Given:** An active existing version of `jira-linter` (v1.0.3) installed on disk.
- **When:** An update to v1.0.4 fails during archive extraction.
- **Then:** The installer rolls back from the `.backup` directory, keeping v1.0.3 active with zero data loss or corrupted directory state.

### AC-P5-4-05: User Capability Permission Consent Flow
- **Given:** A plugin requesting `git_read` and `commit_hook`.
- **When:** The user initiates installation.
- **Then:** Primer UI displays the Permission Review Modal enumerating the specific capabilities, requiring explicit user confirmation before executing download.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.6.0-extensible
