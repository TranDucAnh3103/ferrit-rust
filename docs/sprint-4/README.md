# Sprint 4 — Cross-Platform & Enterprise Distribution (v2.5) Master Guide
**Document Identifier:** SPRINT4-MASTER-SPEC  
**Target Milestone:** Release v2.5.0-enterprise  
**Timeline:** 15 Days (3 Weeks, 5 Phases)  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  

---

## 1. Executive Summary & Sprint Mission

Sprint 4 transitions **GitUpload Desktop** from a Windows-first client into a tier-1 **Universal Cross-Platform & Enterprise-Hardened Suite**. It delivers native compilation, credential isolation, and signed distribution across **macOS (Apple Silicon M-series & Intel x86_64)** and **Linux (Ubuntu, Debian, Fedora, Arch via AppImage, Flatpak, and Snap)**, alongside enterprise-grade air-gapped corporate deployments and cryptographically signed delta auto-updaters.

```
+--------------------------------------------------------------------------------------------------+
|                                    Sprint 4 Architecture Topology                                |
|                                                                                                  |
|  +----------------------------+  +----------------------------+  +----------------------------+  |
|  |          Phase 1           |  |          Phase 2           |  |          Phase 3           |  |
|  |   macOS Universal Binary   |  |     Linux Desktop Suite    |  |    Tauri Auto-Updater      |  |
|  |  (Apple Silicon + Keychain)|  | (AppImage/Flatpak/libsecret|  | (Minisign / 3 Channels)   |  |
|  +-------------+--------------+  +-------------+--------------+  +-------------+--------------+  |
|                |                               |                               |                 |
|                +-------------------------------+-------------------------------+                 |
|                                                |                                                 |
|                                 +--------------v---------------+                                 |
|                                 |           Phase 4            |                                 |
|                                 |  Enterprise Air-Gap & Policy |                                 |
|                                 |  (GPO / Proxy / No-Telemetry)|                                 |
|                                 +--------------+---------------+                                 |
|                                                |                                                 |
|                                 +--------------v---------------+                                 |
|                                 |           Phase 5            |                                 |
|                                 |  Multi-OS Matrix QA & Signoff|                                 |
|                                 +------------------------------+                                 |
+--------------------------------------------------------------------------------------------------+
```

---

## 2. Sprint 4 Phase Breakdown & Index

| Phase | Title | Focus & Core Deliverables | Output Document |
| :--- | :--- | :--- | :--- |
| **Phase 1** | macOS Apple Silicon & Universal Port | Universal macOS binary (`aarch64` + `x86_64`), Apple Keychain Services integration, `.dmg` packaging, Hardened Runtime, and Apple Notarization (`notarytool`). | [phase-1-macos-apple-silicon-port.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/phase-1-macos-apple-silicon-port.md) |
| **Phase 2** | Linux Desktop Suite | Linux AppImage, Flatpak (`org.gitupload.GitUpload`), Snapcraft manifests, D-Bus Secret Service (`libsecret`), Wayland/X11 DPI auto-detection. | [phase-2-linux-desktop-suite.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/phase-2-linux-desktop-suite.md) |
| **Phase 3** | Tauri Auto-Updater & Channel Routing | Cryptographically signed delta updates via `minisign`, 3 release channels (`stable`, `beta`, `nightly`), background download with zero-downtime hot-swap. | [phase-3-tauri-auto-updater.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/phase-3-tauri-auto-updater.md) |
| **Phase 4** | Enterprise Air-Gap & Policy Governance | Strict offline air-gap mode, corporate NTLM/Kerberos proxy traversal, Windows GPO / registry policies (`HKLM\Software\Policies\GitUpload`), zero-telemetry enforcement. | [phase-4-enterprise-airgap-mode.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/phase-4-enterprise-airgap-mode.md) |
| **Phase 5** | Multi-OS CI/CD Verification & Packaging | 3-OS GitHub Actions matrix (Windows 11, macOS 14 Sonoma, Ubuntu 22.04), installer payload verification, packaging sign-off. | [phase-5-sprint4-qa-verification.md](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-4/phase-5-sprint4-qa-verification.md) |

---

## 3. Definition of Ready (DoR) & Definition of Done (DoD)

### Definition of Ready (DoR):
- [x] Sprint 1, 2, and 3 deliverables passing all unit, integration, and benchmark tests.
- [x] Apple Developer ID certificates and App Store Connect API keys configured in GitHub Secrets.
- [x] Flatpak runtime (`org.freedesktop.Platform//23.08`) and Snapcraft build environments initialized.
- [x] Minisign keypair generated for production auto-update payload verification.

### Definition of Done (DoD):
- [x] Compiled binaries execute natively on Apple Silicon (M1/M2/M3/M4) and Intel x86_64 with <28MB idle RAM.
- [x] Linux distributions (Ubuntu, Fedora, Arch) pass GUI test with active `libsecret` keyring encryption.
- [x] Silent auto-updater completes background verification and promptless stage on Windows/macOS.
- [x] Air-gap mode verified with zero outbound network calls under Wireshark / network sandbox inspection.
- [x] All 5 Phase specifications reviewed, approved, and verified by `@verifier`.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.5.0-enterprise
