# Sprint 4 — Phase 1: macOS Apple Silicon Port & Universal Distribution Specification
**Document Identifier:** SPRINT4-PHASE1-MACOS-PORT  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Native Apple Silicon (`aarch64`) & Intel (`x86_64`) Universal macOS Architecture, Apple Keychain Services, `.dmg` Packaging, and Notarization  
**Target Crates:** `gitupload-security`, `gitupload-app`, `src-tauri/`  
**Timeline:** Days 1–3 (Sprint 4)  

---

## 1. Phase Overview & Technical Strategy

Phase 1 establishes tier-1 macOS desktop compatibility. The system compiles as an optimized macOS Universal 2 binary combining native ARM64 (`aarch64-apple-darwin` for M1/M2/M3/M4 Macs) and x86_64 (`x86_64-apple-darwin` for legacy Intel Macs). It replaces Windows Credential Manager with the native Apple Keychain Services via the `security-framework` crate, adjusts window chrome to respect macOS traffic light buttons, and packages into a notarized, drag-and-drop `.dmg` distribution image.

---

## 2. Granular Task Breakdown

### Task TASK-P4-1-01: Universal macOS Binary Build Matrix
- **Task Identifier:** `TASK-P4-1-01`
- **Component:** `.github/workflows/release-macos.yml`, `Cargo.toml`
- **SRS Traceability:** `NFR-PORT-002`, `NFR-PERF-001`

#### Implementation Specification
Configure cross-compilation for `aarch64-apple-darwin` and `x86_64-apple-darwin`, binding `libgit2` statically with SecureTransport / OpenSSL and bundling via `lipo`:

```yaml
# .github/workflows/release-macos.yml snippet
- name: Compile Universal Binary
  run: |
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    mkdir -p target/universal-apple-darwin/release
    lipo -create \
      target/aarch64-apple-darwin/release/gitupload \
      target/x86_64-apple-darwin/release/gitupload \
      -output target/universal-apple-darwin/release/gitupload
    file target/universal-apple-darwin/release/gitupload
```

---

### Task TASK-P4-1-02: Apple Keychain Services Secure Vault Integration
- **Task Identifier:** `TASK-P4-1-02`
- **Component:** `crates/gitupload-security/src/keychain_macos.rs`
- **SRS Traceability:** `NFR-SEC-001`, `FR-SET-001`

#### Rust Implementation Signature
```rust
// crates/gitupload-security/src/keychain_macos.rs
#[cfg(target_os = "macos")]
use security_framework::passwords::{get_generic_password, set_generic_password, delete_generic_password};
use secrecy::{SecretString, ExposeSecret};
use crate::error::SecurityResult;

pub struct AppleKeychainVault;

impl AppleKeychainVault {
    const SERVICE_NAME: &'static str = "com.gitupload.desktop";

    pub fn store_secret(account: &str, secret: &SecretString) -> SecurityResult<()> {
        set_generic_password(Self::SERVICE_NAME, account, secret.expose_secret().as_bytes())
            .map_err(|e| crate::error::SecurityError::VaultWriteError(e.to_string()))?;
        Ok(())
    }

    pub fn retrieve_secret(account: &str) -> SecurityResult<SecretString> {
        let password_bytes = get_generic_password(Self::SERVICE_NAME, account)
            .map_err(|e| crate::error::SecurityError::SecretNotFound(format!("{}: {}", account, e)))?;
        let secret_str = String::from_utf8(password_bytes)
            .map_err(|e| crate::error::SecurityError::VaultReadError(e.to_string()))?;
        Ok(SecretString::new(secret_str.into()))
    }

    pub fn delete_secret(account: &str) -> SecurityResult<()> {
        delete_generic_password(Self::SERVICE_NAME, account)
            .map_err(|e| crate::error::SecurityError::VaultDeleteError(e.to_string()))?;
        Ok(())
    }
}
```

---

### Task TASK-P4-1-03: Native macOS Primer Chrome & Window Insets
- **Task Identifier:** `TASK-P4-1-03`
- **Component:** `src-tauri/tauri.conf.json`, `ui/src/styles/macos.css`
- **SRS Traceability:** `NFR-USE-001`, `FR-SET-003`

#### Configuration & Styling Guide
On macOS, hide standard window titlebars (`titleBarStyle: "Overlay"`), enabling native macOS traffic light control inset:
```json
// src-tauri/tauri.conf.json (macOS override)
{
  "app": {
    "windows": [
      {
        "title": "GitUpload",
        "titleBarStyle": "Overlay",
        "hiddenTitle": true,
        "trafficLightInset": { "x": 16, "y": 18 }
      }
    ]
  }
}
```

---

### Task TASK-P4-1-04: DMG Disk Image Packaging & Drag-and-Drop Layout
- **Task Identifier:** `TASK-P4-1-04`
- **Component:** `src-tauri/dmg/`, `src-tauri/tauri.conf.json`
- **SRS Traceability:** `NFR-PORT-002`

#### Technical Details
Configure high-resolution retina background (`installer-dmg.png`), 540x380 viewport, and standard symlink to `/Applications`:
```json
"bundle": {
  "macOS": {
    "frameworks": [],
    "minimumSystemVersion": "12.0",
    "signingIdentity": "Developer ID Application: Stephen (TEAMID123)",
    "dmg": {
      "background": "dmg/background.png",
      "windowSize": { "width": 540, "height": 380 },
      "appPosition": { "x": 140, "y": 180 },
      "applicationFolderPosition": { "x": 400, "y": 180 }
    }
  }
}
```

---

### Task TASK-P4-1-05: Apple Hardened Runtime & Notarization Pipeline
- **Task Identifier:** `TASK-P4-1-05`
- **Component:** `src-tauri/Entitlements.plist`, `scripts/notarize-macos.sh`
- **SRS Traceability:** `NFR-SEC-004`, `NFR-PORT-002`

```xml
<!-- src-tauri/Entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.keychain</key>
    <true/>
</dict>
</plist>
```

---

## 3. Strict Acceptance Criteria Matrix (AC-P4-1-xxx)

| Criteria ID | Scenario | Given-When-Then Verification | Verification Command |
| :--- | :--- | :--- | :--- |
| **AC-P4-1-01** | Universal Binary Inspection | **Given** the compiled release binary, **When** inspected with `lipo -info`, **Then** it must report both `arm64` and `x86_64` architecture slices. | `lipo -info target/universal-apple-darwin/release/gitupload` |
| **AC-P4-1-02** | Keychain Token Storage | **Given** user authenticates with GitHub, **When** token is stored, **Then** it must be written to Apple Keychain under `com.gitupload.desktop` and retrieve correctly. | `cargo test -p gitupload-security --test test_macos_keychain` |
| **AC-P4-1-03** | macOS Traffic Light Positioning | **Given** macOS window launch, **When** header renders, **Then** traffic light buttons (close/minimize/zoom) must have 16px horizontal and 18px vertical insets without overlap. | UI Visual Inspection on macOS 14 Sonoma |
| **AC-P4-1-04** | Hardened Runtime Check | **Given** signed application bundle, **When** verified via `codesign`, **Then** output must verify `flags=0x10000(runtime)`. | `codesign --display --verbose=4 GitUpload.app` |
| **AC-P4-1-05** | Notarization Gate | **Given** packaged `.dmg`, **When** submitted to Apple Notary Service via `notarytool`, **Then** notarization status must return `Accepted` with valid ticket stapled. | `xcrun stapler validate GitUpload.dmg` |

---

*Document Author:* `@srs_author`  
*Reviewers:* `@verifier`, `@coordinator`
