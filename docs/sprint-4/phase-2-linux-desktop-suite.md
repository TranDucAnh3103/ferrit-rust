# Sprint 4 — Phase 2: Linux Desktop Suite (AppImage, Flatpak, Snap, libsecret) Specification
**Document Identifier:** SPRINT4-PHASE2-LINUX-SUITE  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Universal Linux Distribution (AppImage, Flatpak, Snap), D-Bus Secret Service, Wayland/X11 Protocol Compatibility, and Desktop Integrations  
**Target Crates:** `gitupload-security`, `gitupload-app`, `packaging/linux/`  
**Timeline:** Days 4–6 (Sprint 4)  

---

## 1. Phase Overview & Technical Strategy

Phase 2 delivers universal, distribution-agnostic Linux desktop packages for Ubuntu, Debian, Fedora, openSUSE, and Arch Linux. It addresses Linux fragmentation by providing **AppImage (zero-install portable binary)**, **Flatpak (sandboxed deployment via Flathub)**, and **Snap (Ubuntu Software Center)**. It integrates directly with the freedesktop.org Secret Service API (`libsecret` over D-Bus) for hardware/GNOME Keyring and KDE KWallet encryption, and enforces native Wayland and X11 display scaling.

---

## 2. Granular Task Breakdown

### Task TASK-P4-2-01: Linux AppImage & Flatpak Bundler Pipelines
- **Task Identifier:** `TASK-P4-2-01`
- **Component:** `packaging/linux/flatpak/org.gitupload.GitUpload.yaml`, `src-tauri/tauri.conf.json`
- **SRS Traceability:** `NFR-PORT-002`

#### Flatpak Manifest Specification
```yaml
# packaging/linux/flatpak/org.gitupload.GitUpload.yaml
app-id: org.gitupload.GitUpload
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
command: gitupload
finish-args:
  - --share=network
  - --share=ipc
  - --socket=fallback-x11
  - --socket=wayland
  - --filesystem=host:ro
  - --filesystem=home
  - --talk-name=org.freedesktop.secrets
build-options:
  append-path: /usr/lib/sdk/rust-stable/bin
modules:
  - name: gitupload
    buildsystem: simple
    build-commands:
      - cargo build --release --offline
      - install -Dm755 target/release/gitupload /app/bin/gitupload
      - install -Dm644 packaging/linux/org.gitupload.GitUpload.desktop /app/share/applications/org.gitupload.GitUpload.desktop
      - install -Dm644 packaging/linux/icons/128x128.png /app/share/icons/hicolor/128x128/apps/org.gitupload.GitUpload.png
```

---

### Task TASK-P4-2-02: D-Bus Secret Service (`libsecret`) Linux Vault
- **Task Identifier:** `TASK-P4-2-02`
- **Component:** `crates/gitupload-security/src/secret_service_linux.rs`
- **SRS Traceability:** `NFR-SEC-001`, `FR-SET-001`

#### Rust Implementation Signature
```rust
// crates/gitupload-security/src/secret_service_linux.rs
#[cfg(target_os = "linux")]
use secret_service::{SecretService, EncryptionType};
use secrecy::{SecretString, ExposeSecret};
use std::collections::HashMap;
use crate::error::SecurityResult;

pub struct LinuxSecretServiceVault;

impl LinuxSecretServiceVault {
    const COLLECTION: &'static str = "default";
    const APP_LABEL: &'static str = "GitUpload Desktop";

    pub async fn store_secret(account: &str, secret: &SecretString) -> SecurityResult<()> {
        let ss = SecretService::connect(EncryptionType::Dh).await
            .map_err(|e| crate::error::SecurityError::VaultWriteError(e.to_string()))?;
        let collection = ss.get_default_collection().await
            .map_err(|e| crate::error::SecurityError::VaultWriteError(e.to_string()))?;

        let mut attributes = HashMap::new();
        attributes.insert("application", "gitupload");
        attributes.insert("account", account);

        collection.create_item(
            &format!("{}: {}", Self::APP_LABEL, account),
            attributes,
            secret.expose_secret().as_bytes(),
            true, // replace existing
            "text/plain",
        ).await.map_err(|e| crate::error::SecurityError::VaultWriteError(e.to_string()))?;

        Ok(())
    }

    pub async fn retrieve_secret(account: &str) -> SecurityResult<SecretString> {
        let ss = SecretService::connect(EncryptionType::Dh).await
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;
        let collection = ss.get_default_collection().await
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;

        let mut attributes = HashMap::new();
        attributes.insert("application", "gitupload");
        attributes.insert("account", account);

        let items = collection.search_items(attributes).await
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;
        let item = items.first().ok_or_else(|| {
            crate::error::SecurityError::SecretNotFound(account.to_string())
        })?;

        let secret_bytes = item.get_secret().await
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;
        let secret_str = String::from_utf8(secret_bytes)
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;

        Ok(SecretString::new(secret_str.into()))
    }
}
```

---

### Task TASK-P4-2-03: Wayland & X11 DPI Auto-Scaling & Protocol Adaptation
- **Task Identifier:** `TASK-P4-2-03`
- **Component:** `crates/gitupload-app/src/main.rs`, `packaging/linux/env.sh`
- **SRS Traceability:** `NFR-USE-001`, `NFR-PERF-004`

#### Implementation Guide
Ensure WebKitGTK respects system display scaling without blurry fonts under fractional scaling on GNOME/KDE:
```rust
// Pre-initialization hook in main.rs
#[cfg(target_os = "linux")]
fn configure_linux_display_backend() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("GDK_BACKEND").is_err() {
        // Prefer Wayland with X11 fallback
        std::env::set_var("GDK_BACKEND", "wayland,x11");
    }
    // Enable WebKitGTK hardware acceleration and fractional DPI scaling
    std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
}
```

---

### Task TASK-P4-2-04: Linux Desktop Entry & MIME Associations
- **Task Identifier:** `TASK-P4-2-04`
- **Component:** `packaging/linux/org.gitupload.GitUpload.desktop`
- **SRS Traceability:** `NFR-PORT-002`

```ini
# packaging/linux/org.gitupload.GitUpload.desktop
[Desktop Entry]
Name=GitUpload
GenericName=Git Desktop Client
Comment=GitHub-Fidelity Git Automation & Productivity Suite
Exec=gitupload %U
Icon=org.gitupload.GitUpload
Terminal=false
Type=Application
Categories=Development;RevisionControl;IDE;
MimeType=x-scheme-handler/gitupload;inode/directory;
Keywords=git;github;vcs;upload;code;
StartupWMClass=gitupload
```

---

## 3. Strict Acceptance Criteria Matrix (AC-P4-2-xxx)

| Criteria ID | Scenario | Given-When-Then Verification | Verification Command |
| :--- | :--- | :--- | :--- |
| **AC-P4-2-01** | AppImage Execution on Clean Ubuntu 22.04 | **Given** an unconfigured Ubuntu 22.04 VM, **When** `GitUpload.AppImage` is executed, **Then** application must render interactive window in <100ms. | `./GitUpload.AppImage --version` |
| **AC-P4-2-02** | D-Bus Secret Service Storage | **Given** GNOME Keyring active, **When** GitHub token is stored, **Then** it must be encrypted in D-Bus Secret Service and queryable via `secret-tool`. | `secret-tool lookup application gitupload account default` |
| **AC-P4-2-03** | Wayland Native Window Protocol | **Given** an active Wayland session, **When** app starts, **Then** window must register under native Wayland protocol without XWayland fallback. | `WAYLAND_DEBUG=1 gitupload \| grep "wl_surface"` |
| **AC-P4-2-04** | Flatpak Sandbox Isolation | **Given** Flatpak installation, **When** verified via `flatpak run`, **Then** filesystem access must be restricted to home directory without root escalation. | `flatpak run org.gitupload.GitUpload` |
| **AC-P4-2-05** | Memory Overhead on Linux | **Given** idle state on Linux, **When** RSS memory measured via `ps`, **Then** resident memory must consume **< 28 MB**. | `ps -o rss,command -C gitupload` |

---

*Document Author:* `@srs_author`  
*Reviewers:* `@verifier`, `@coordinator`
