#![forbid(unsafe_code)]
//! Security subsystem: OS Credential Vault (`keyring` + `secrecy` + `zeroize`), SSH Agent, and real-time log sanitization.

pub mod hmac;
pub mod keyring;
pub mod platform_vault;
pub mod remote_vault;
pub mod sanitizer;
pub mod scanner;
pub mod ssh;

pub use hmac::{sign_payload, verify_payload_signature};
pub use keyring::{KeyringManager, SecretInput};
pub use platform_vault::PlatformVault;
pub use remote_vault::RemoteVaultResolver;
pub use sanitizer::{mask_sensitive_tokens, sanitize_log_message};
pub use scanner::StaticSecretScanner;
pub use ssh::{
    compute_fingerprint_sha256, generate_ssh_keypair, GeneratedSshKeypair, NativeSshAgent,
    SshVault, DEFAULT_WINDOWS_SSH_AGENT_PIPE,
};
