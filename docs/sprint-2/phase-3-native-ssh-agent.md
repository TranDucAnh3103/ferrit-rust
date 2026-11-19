# Sprint 2 — Phase 3: Pure-Rust Native SSH Agent & Cryptographic Key Management Specification
**Document Identifier:** SPRINT2-PHASE3-NATIVE-SSH-AGENT  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Zero-Dependency In-Process Key Generation (ED25519, RSA 4096), In-Memory SSH Agent Daemon (Windows Named Pipe Compatibility), OS Keyring Vault Integration, and Automated API Public Key Registration  
**Target Crates:** `gitupload-security`, `gitupload-git`, `gitupload-github`, `gitupload-app`, `ui/`  
**Timeline:** Days 11–15 (Sprint 2 Cryptographic Infrastructure)  

---

## 1. Phase Overview & Objectives

Managing SSH keys is notoriously error-prone for developers and students on Windows: mismatched OpenSSH vs Pageant paths, incorrect `~/.ssh/config` file permissions (causing silent Git push rejections), and insecure plaintext private key storage.

Phase 3 implements a **Zero-Dependency Native SSH Subsystem** written purely in Rust. It eliminates external OpenSSH client requirements by generating cryptographic keys directly in-process, managing an in-memory SSH Agent daemon listening on the standard Windows OpenSSH Named Pipe, and automatically uploading generated public keys to GitHub and GitLab via REST APIs with zero manual copy-pasting.

### Core Objectives:
1. **In-Process Cryptographic Key Generation:** Generate industry-standard **ED25519** (high speed, 256-bit elliptic curve) and **RSA 4096** key pairs directly in Rust without invoking `ssh-keygen.exe`.
2. **Encrypted In-Memory Vault Hygiene:** Encrypt generated private keys using Windows DPAPI via `KeyringManager`. Keep private keys strictly in memory (`secrecy::SecretString`), never writing unencrypted `.id_ed25519` files to disk unless explicitly exported by the user.
3. **In-Memory Windows Named Pipe SSH Agent Daemon:** Expose an asynchronous agent daemon on `\\.\pipe\openssh-ssh-agent`. Answer standard SSH Agent protocol requests (`SSH2_AGENTC_REQUEST_IDENTITIES`, `SSH2_AGENTC_SIGN_REQUEST`) from `libgit2` or CLI `git.exe`.
4. **Automated API Public Key Registration:** Use `gitupload-github` (and GitLab REST API) to register the generated public key (`POST /user/keys`) with automated title tagging (e.g. `GitUpload Desktop [Hostname]`), instantly authorizing Git push operations.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 3 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |        TASK-S2P3-01         |  |         TASK-S2P3-02          |  |       TASK-S2P3-03      |  |
|  | Cryptographic Key Models    |  |  Pure-Rust Key Generator      |  | In-Memory SSH Agent     |  |
|  | (ED25519, RSA, Fingerprint) |  |  (ed25519-dalek, rsa crate)   |  | (Windows Named Pipe)    |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |    Native SSH Subsystem      |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |        TASK-S2P3-04         |   |         TASK-S2P3-05          |   |       TASK-S2P3-06     |  |
|  | Windows Vault Key Storage   |   | Automated API Key Upload      |   | Primer UI SSH Manager  |  |
|  | (DPAPI / Zeroize Hygiene)   |   | (GitHub / GitLab REST API)    |   | (1-Click Key Provision)|  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-S2P3-01: Cryptographic Key Models & SSH Protocol Definitions

- **Task Identifier:** `TASK-S2P3-01`
- **Component:** `crates/gitupload-core/src/models/ssh.rs`
- **SRS Traceability:** `FR-GIT-019`, `NFR-SEC-001`

#### Implementation Specification
Define serializable structures representing SSH identities and agent protocols:

```rust
// crates/gitupload-core/src/models/ssh.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SshKeyAlgorithm {
    Ed25519,
    Rsa4096,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKeyDescriptor {
    pub key_id: String,
    pub title: String,
    pub algorithm: SshKeyAlgorithm,
    pub public_key: String,
    pub fingerprint_sha256: String,
    pub created_at: DateTime<Utc>,
    pub is_registered_on_github: bool,
}
```

---

### Task TASK-S2P3-02: Pure-Rust Key Pair Generator

- **Task Identifier:** `TASK-S2P3-02`
- **Component:** `crates/gitupload-security/src/ssh/keygen.rs`
- **SRS Traceability:** `FR-GIT-019`, `NFR-SEC-001`

#### Implementation Specification
Implement in-process key pair creation using pure-Rust cryptographic libraries:
1. **ED25519:** Uses `ed25519-dalek` to generate 32-byte secret seed and derive 32-byte public key.
2. Format OpenSSH public key string: `ssh-ed25519 <base64_pubkey> <comment>`.
3. Format OpenSSH private key PEM (PKCS#8 with Ed25519 OID).
4. Compute SHA256 fingerprint: `SHA256:<base64_digest>`.

```rust
// crates/gitupload-security/src/ssh/keygen.rs
use ed25519_dalek::{SigningKey, VerifyingKey};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::ssh::{SshKeyAlgorithm, SshKeyDescriptor};
use rand::rngs::OsRng;
use secrecy::SecretString;
use sha2::{Digest, Sha256};

pub struct SshKeyGenerator;

impl SshKeyGenerator {
    pub fn generate_ed25519(comment: &str) -> GitUploadResult<(SshKeyDescriptor, SecretString)> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let pub_bytes = verifying_key.as_bytes();
        let pub_b64 = base64::encode(pub_bytes);
        let public_key = format!("ssh-ed25519 {} {}", pub_b64, comment);

        let mut hasher = Sha256::new();
        hasher.update(pub_bytes);
        let fingerprint = format!("SHA256:{}", base64::encode(hasher.finalize()));

        let priv_bytes = signing_key.to_bytes();
        let private_pem = format!("-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----", base64::encode(priv_bytes));

        let descriptor = SshKeyDescriptor {
            key_id: uuid::Uuid::new_v4().to_string(),
            title: comment.to_string(),
            algorithm: SshKeyAlgorithm::Ed25519,
            public_key,
            fingerprint_sha256: fingerprint,
            created_at: chrono::Utc::now(),
            is_registered_on_github: false,
        };

        Ok((descriptor, SecretString::new(private_pem.into())))
    }
}
```

---

### Task TASK-S2P3-03: In-Memory Windows Named Pipe SSH Agent Daemon

- **Task Identifier:** `TASK-S2P3-03`
- **Component:** `crates/gitupload-security/src/ssh/agent.rs`
- **SRS Traceability:** `FR-GIT-019`, `NFR-SEC-002`

#### Implementation Specification
Implement an asynchronous Named Pipe listener:
- Path: `\\.\pipe\openssh-ssh-agent` (fallback to `\\.\pipe\gitupload-ssh-agent`).
- Decodes SSH Agent Protocol messages:
  - `SSH2_AGENTC_REQUEST_IDENTITIES (11)`: Returns loaded public keys.
  - `SSH2_AGENTC_SIGN_REQUEST (13)`: Performs in-memory signature using the corresponding private key without exposing private key bytes.
- Runs inside a background Tokio task.

---

### Task TASK-S2P3-04: DPAPI Vault Private Key Storage

- **Task Identifier:** `TASK-S2P3-04`
- **Component:** `crates/gitupload-security/src/ssh/vault.rs`
- **SRS Traceability:** `NFR-SEC-001`, `NFR-SEC-002`

#### Implementation Specification
Integrate private key persistence with Windows Credential Manager:
- Target: `GitUploadDesktop/ssh_key:<key_id>`.
- Private keys are encrypted by Windows DPAPI tied to the logged-in Windows user account.
- Zero plaintext keys stored on filesystem.

---

### Task TASK-S2P3-05: Automated Public Key Upload to GitHub & GitLab APIs

- **Task Identifier:** `TASK-S2P3-05`
- **Component:** `crates/gitupload-github/src/ssh_keys.rs`
- **SRS Traceability:** `FR-GH-001`, `FR-GIT-019`

#### Implementation Specification
Automated registration of the public key:
- Calls GitHub REST API `POST https://api.github.com/user/keys`.
- Payload: `{"title": "GitUpload Desktop - StephenLap", "key": "ssh-ed25519 AAAA..."}`.
- Handles duplicate key detection (`422 Unprocessable Entity - key already in use`).
- Marks `is_registered_on_github = true` in SQLite.

---

### Task TASK-S2P3-06: Primer UI SSH Key Management Component

- **Task Identifier:** `TASK-S2P3-06`
- **Component:** `ui/src/views/SshKeysView.tsx`, `ui/src/components/SshKeyCard.tsx`
- **SRS Traceability:** `FR-SET-001`, `FR-USE-001`

#### Implementation Specification
GitHub Primer-styled view for SSH key lifecycle:
- "Generate New SSH Key" 1-click modal with choice of ED25519 (Recommended) or RSA 4096.
- List of active keys with SHA256 fingerprint, creation date, and status badges.
- 1-click "Add to GitHub" and "Add to GitLab" action buttons.
- "Copy Public Key" button with toast notification.

---

## 3. Strict Acceptance Criteria (AC-xxx)

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-S2P3-01** | User requests new ED25519 key pair | Calling `SshKeyGenerator::generate_ed25519()` | Valid OpenSSH public key and SHA256 fingerprint generated in < 5ms without `ssh-keygen.exe` | `cargo test -p gitupload-security test_keygen_ed25519` |
| **AC-S2P3-02** | Generated private key | Storing via `SshKeyVault::store_key()` | Encrypted in Windows Credential Manager; zero plaintext files created in `.ssh/` | `cargo test -p gitupload-security test_ssh_vault_storage` |
| **AC-S2P3-03** | In-memory agent running on Named Pipe | Client sends `SSH2_AGENTC_REQUEST_IDENTITIES` | Responds with list of loaded public keys following RFC draft-miller-ssh-agent | `cargo test -p gitupload-security test_named_pipe_agent` |
| **AC-S2P3-04** | Valid GitHub token and generated public key | Calling `GitHubClient::register_ssh_key()` | Key registered on GitHub API with status 201 Created and bound to account | `cargo test -p gitupload-github test_api_upload_ssh_key` |
| **AC-S2P3-05** | Git remote using `git@github.com:...` URL | Executing push using in-memory SSH agent | Authenticates seamlessly without password prompts or external pageant daemon | `cargo test -p gitupload-git test_ssh_transport_push` |
| **AC-S2P3-06** | SSH Key Management UI | User clicks "Generate & Link to GitHub" | Automatically generates ED25519 key, saves to vault, uploads to GitHub, and displays green badge in < 2s | UI Integration Test |

---

## 4. Definition of Done (DoD) Checklist for Phase 3

- [x] Pure-Rust key generation compiles clean without external C crypto dependencies.
- [x] `#![forbid(unsafe_code)]` enforced across all cryptographic modules.
- [x] 100% of SSH unit and agent protocol tests pass (`cargo test -p gitupload-security ssh`).
- [x] Public key successfully uploaded to mock GitHub API and validated.
- [x] Clippy reports zero warnings under `cargo clippy --all-targets -- -D warnings`.
