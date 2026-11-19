# Sprint 4 — Phase 4: Enterprise Air-Gap Mode & Policy Governance Specification
**Document Identifier:** SPRINT4-PHASE4-ENTERPRISE-AIRGAP  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Offline Air-Gapped Network Isolation, Corporate Proxy Support (NTLM/Kerberos/Custom CAs), Windows Group Policy (GPO) Registry Enforcement, and Self-Hosted Git Remotes  
**Target Crates:** `gitupload-core`, `gitupload-security`, `gitupload-github`, `gitupload-app`  
**Timeline:** Days 10–12 (Sprint 4)  

---

## 1. Phase Overview & Technical Strategy

Phase 4 hardens GitUpload Desktop for highly secure financial, defense, and corporate enterprise networks. It introduces a **Strict Air-Gap Mode** that physically blocks all outbound telemetry and external cloud services, routes traffic through corporate authenticating forward proxies (with custom enterprise root CA certificates), and obeys system-wide policies enforced by Windows Group Policy (`HKLM\Software\Policies\GitUpload`) or Linux/macOS `/etc/gitupload/policy.json`.

---

## 2. Granular Task Breakdown

### Task TASK-P4-4-01: Strict Air-Gap Mode Enforcement Guard
- **Task Identifier:** `TASK-P4-4-01`
- **Component:** `crates/gitupload-core/src/airgap.rs`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-005`

#### Rust Implementation Signature
```rust
// crates/gitupload-core/src/airgap.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub struct AirGapGuard;

static AIRGAP_ENFORCED: AtomicBool = AtomicBool::new(false);

impl AirGapGuard {
    pub fn enable() {
        AIRGAP_ENFORCED.store(true, Ordering::SeqCst);
    }

    pub fn is_enabled() -> bool {
        AIRGAP_ENFORCED.load(Ordering::SeqCst)
    }

    pub fn assert_network_allowed(destination_host: &str, is_local_network: bool) -> Result<(), &'static str> {
        if Self::is_enabled() && !is_local_network {
            return Err("Operation blocked: Strict Enterprise Air-Gap Mode is enforced.");
        }
        Ok(())
    }
}
```

---

### Task TASK-P4-4-02: Corporate Authenticating Proxy & Custom CA Cert Manager
- **Task Identifier:** `TASK-P4-4-02`
- **Component:** `crates/gitupload-github/src/proxy.rs`
- **SRS Traceability:** `NFR-PORT-001`, `NFR-SEC-002`

#### Rust Implementation Signature
```rust
// crates/gitupload-github/src/proxy.rs
use reqwest::{Client, Proxy, Certificate};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct CorporateProxyConfig {
    pub proxy_url: String,
    pub custom_ca_cert_path: Option<String>,
    pub bypass_hosts: Vec<String>,
}

impl CorporateProxyConfig {
    pub fn build_http_client(&self) -> Result<Client, Box<dyn std::error::Error>> {
        let mut builder = Client::builder();

        // 1. Configure corporate proxy
        if !self.proxy_url.is_empty() {
            let proxy = Proxy::all(&self.proxy_url)?;
            builder = builder.proxy(proxy);
        }

        // 2. Load custom corporate Root CA cert (PEM format)
        if let Some(ca_path) = &self.custom_ca_cert_path {
            let pem_bytes = std::fs::read(ca_path)?;
            let cert = Certificate::from_pem(&pem_bytes)?;
            builder = builder.add_root_certificate(cert);
        }

        Ok(builder.build()?)
    }
}
```

---

### Task TASK-P4-4-03: Windows Group Policy (GPO) Registry Enforcement
- **Task Identifier:** `TASK-P4-4-03`
- **Component:** `crates/gitupload-app/src/platform/gpo_windows.rs`
- **SRS Traceability:** `NFR-SEC-001`, `FR-SET-002`

#### Windows Registry Policy Schema
Policies set under `HKLM\Software\Policies\GitUpload` override local user settings permanently:
- `DisableCloudAI` (DWORD: 1 = Enforce local or disabled AI)
- `ForceAirGapMode` (DWORD: 1 = Lock airgap mode)
- `CorporateProxyUrl` (SZ: "http://proxy.corp.internal:8080")
- `AllowedGitHubDomains` (MULTI_SZ: "github.corp.internal", "ghe.defense.gov")

```rust
// crates/gitupload-app/src/platform/gpo_windows.rs
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Registry::{RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, KEY_READ};

pub struct EnterprisePolicyManager;

impl EnterprisePolicyManager {
    #[cfg(target_os = "windows")]
    pub fn is_cloud_ai_disabled_by_gpo() -> bool {
        // Reads HKLM\Software\Policies\GitUpload\DisableCloudAI
        // Returns true if DWORD == 1
        true // Stub implementation
    }
}
```

---

### Task TASK-P4-4-04: Cross-Platform File Policy Provider (`/etc/gitupload/policy.json`)
- **Task Identifier:** `TASK-P4-4-04`
- **Component:** `crates/gitupload-core/src/policy_file.rs`
- **SRS Traceability:** `NFR-PORT-002`

```json
// /etc/gitupload/policy.json (Linux / macOS system-wide override)
{
  "enterprise": {
    "organization_name": "Acme Defense Corp",
    "strict_airgap": true,
    "disable_telemetry": true,
    "allowed_remotes": ["https://git.acme.internal/*"],
    "enforced_commit_sign": true
  }
}
```

---

## 3. Strict Acceptance Criteria Matrix (AC-P4-4-xxx)

| Criteria ID | Scenario | Given-When-Then Verification | Verification Command |
| :--- | :--- | :--- | :--- |
| **AC-P4-4-01** | Air-Gap Network Interception | **Given** `AirGapGuard::enable()` active, **When** any outbound HTTP call to public internet is attempted, **Then** call must be rejected immediately before socket creation. | `cargo test -p gitupload-core --test test_airgap_interception` |
| **AC-P4-4-02** | Corporate Self-Signed CA Trust | **Given** custom corporate root CA provided, **When** connecting to internal HTTPS Git server, **Then** TLS handshake must succeed without certificate authority errors. | `cargo test -p gitupload-github --test test_custom_ca_cert` |
| **AC-P4-4-03** | Windows GPO Override | **Given** `DisableCloudAI=1` in `HKLM\Software\Policies\GitUpload`, **When** application loads, **Then** Cloud AI controls must be disabled and display lock icon in UI. | E2E Windows GPO VM Test |
| **AC-P4-4-04** | Telemetry Zero-Outbound Audit | **Given** airgap mode enabled, **When** packet capture run via Wireshark during 30 min session, **Then** zero packets shall be sent to external IP addresses. | Wireshark Packet Inspection |

---

*Document Author:* `@srs_author`  
*Reviewers:* `@verifier`, `@coordinator`
