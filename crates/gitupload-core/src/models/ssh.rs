//! Pure-Rust SSH key identity descriptor models (metadata only; private keys live in OS Keyring).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported asymmetric cryptography algorithms for SSH keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SshKeyAlgorithm {
    /// Edwards-curve Digital Signature Algorithm (recommended).
    Ed25519,
    /// 4096-bit RSA keypair with SHA-256 OpenSSH signatures.
    Rsa4096,
}

/// Non-sensitive metadata describing a managed SSH keypair.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SshKeyDescriptor {
    /// Unique UUID identifying the key inside OS Credential Vault & SQLite.
    pub key_id: String,
    /// Display title / key comment (`GitUpload Desktop - Host`).
    pub title: String,
    /// Cryptographic algorithm (`ed25519` or `rsa4096`).
    pub algorithm: SshKeyAlgorithm,
    /// OpenSSH public key line (`ssh-ed25519 AAAA... comment`).
    pub public_key: String,
    /// Cryptographic fingerprint (`SHA256:...`).
    pub fingerprint_sha256: String,
    /// UTC creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Whether this public key has been registered via GitHub/GitLab API.
    pub is_registered_on_github: bool,
}
