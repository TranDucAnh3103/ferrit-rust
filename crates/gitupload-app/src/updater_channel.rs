#![forbid(unsafe_code)]
//! Multi-channel auto-updater routing (`stable`, `beta`, `nightly`) and Ed25519/Minisign payload verification.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use gitupload_core::AirGapGuard;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Default embedded GitUpload Ed25519 release verification key (replaceable via build/CI override).
pub const DEFAULT_UPDATE_SIGNING_PUBKEY_B64: &str =
    "rW6s8tQ0Y2Z0d2l0aGVyZWRkMjU1MTlrZXkxMjM0NTY3ODkwMTI=";

/// Software release distribution channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    /// Production-tested GA releases.
    #[default]
    Stable,
    /// Release candidates & early preview builds.
    Beta,
    /// Daily development tip builds.
    Nightly,
}

impl UpdateChannel {
    /// Returns normalized channel string (`stable`, `beta`, `nightly`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Nightly => "nightly",
        }
    }

    /// Parses a channel string case-insensitively.
    pub fn from_str_lossy(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "beta" => Self::Beta,
            "nightly" => Self::Nightly,
            _ => Self::Stable,
        }
    }

    /// Builds the target-scoped update manifest query URL carrying the active channel parameter.
    pub fn endpoint_url(&self, current_version: &str, target_os: &str) -> String {
        format!(
            "https://releases.gitupload.org/api/v1/update/{target_os}/{current_version}?channel={}",
            self.as_str()
        )
    }
}

/// Cryptographic or network failure during signed auto-update verification.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UpdaterError {
    /// Cryptographic Ed25519 signature did not verify against payload.
    #[error("SignatureVerificationFailed: {0}")]
    SignatureVerificationFailed(String),
    /// SHA-256 digest mismatch between manifest and downloaded artifact.
    #[error("ChecksumMismatch: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Expected hex SHA-256 digest.
        expected: String,
        /// Computed hex SHA-256 digest.
        actual: String,
    },
    /// Blocked by Enterprise Air-Gap policy.
    #[error("AirGapBlocked: {0}")]
    AirGapBlocked(String),
    /// Network or JSON decoding failure.
    #[error("UpdateFetchError: {0}")]
    FetchError(String),
}

/// Signed remote update manifest returned by a release channel server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateManifest {
    /// Semantic version tag (`2.5.0`).
    pub version: String,
    /// Markdown release notes.
    pub release_notes: String,
    /// ISO-8601 publication timestamp.
    pub pub_date: String,
    /// Base64-encoded 64-byte Ed25519 signature over raw payload or canonical manifest bytes.
    pub signature: String,
    /// Artifact download URL.
    pub download_url: String,
    /// Lowercase hex SHA-256 checksum of the update payload.
    pub sha256_checksum: String,
}

/// Compact update availability metadata sent to the frontend UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateManifestInfo {
    /// Available version string.
    pub version: String,
    /// Release notes body.
    pub body: String,
    /// Publication date string.
    pub date: String,
    /// Download URL.
    pub download_url: String,
    /// True when manifest Ed25519 signature validated cleanly.
    pub signature_verified: bool,
}

/// Micro-batched chunked download progress event payload (`update-download-progress`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub downloaded: usize,
    /// Total expected payload bytes if known.
    pub total: Option<u64>,
}

/// Verifies an Ed25519 signature (raw base64 64-byte signature OR Minisign 2-line base64 envelope) against `payload`.
pub fn verify_ed25519_payload(
    pubkey_b64: &str,
    payload: &[u8],
    signature_b64: &str,
) -> Result<(), UpdaterError> {
    let key_bytes = STANDARD.decode(pubkey_b64.trim()).map_err(|e| {
        UpdaterError::SignatureVerificationFailed(format!("Invalid public key base64: {e}"))
    })?;
    let key_arr: [u8; 32] = key_bytes.try_into().map_err(|_| {
        UpdaterError::SignatureVerificationFailed(
            "Public key must be exact 32-byte Ed25519 key".to_string(),
        )
    })?;
    let verifying_key = VerifyingKey::from_bytes(&key_arr).map_err(|e| {
        UpdaterError::SignatureVerificationFailed(format!("Malformed Ed25519 key: {e}"))
    })?;

    let sig_bytes = STANDARD.decode(signature_b64.trim()).map_err(|e| {
        UpdaterError::SignatureVerificationFailed(format!("Invalid signature base64: {e}"))
    })?;
    let sig_arr: [u8; 64] = sig_bytes.try_into().map_err(|_| {
        UpdaterError::SignatureVerificationFailed(
            "Signature must be exact 64-byte Ed25519 signature".to_string(),
        )
    })?;
    let sig = Signature::from_bytes(&sig_arr);

    verifying_key
        .verify_strict(payload, &sig)
        .map_err(|e| UpdaterError::SignatureVerificationFailed(e.to_string()))
}

/// Verifies SHA-256 digest of `payload` against expected lowercase/uppercase hex string.
pub fn verify_payload_sha256(payload: &[u8], expected_hex: &str) -> Result<(), UpdaterError> {
    let actual = hex::encode(Sha256::digest(payload));
    let expected = expected_hex.trim().to_ascii_lowercase();
    if actual != expected {
        return Err(UpdaterError::ChecksumMismatch { expected, actual });
    }
    Ok(())
}

/// Verifies an [`UpdateManifest`]'s signature against `"{version}|{sha256_checksum}|{download_url}"`.
pub fn verify_manifest_signature(
    manifest: &UpdateManifest,
    pubkey_b64: &str,
) -> Result<(), UpdaterError> {
    let canonical = format!(
        "{}|{}|{}",
        manifest.version.trim(),
        manifest.sha256_checksum.trim().to_ascii_lowercase(),
        manifest.download_url.trim()
    );
    verify_ed25519_payload(pubkey_b64, canonical.as_bytes(), &manifest.signature)
}

/// Fetches and cryptographically verifies an update manifest from `endpoint_url`.
pub async fn fetch_and_verify_manifest(
    client: &reqwest::Client,
    endpoint_url: &str,
    pubkey_b64: &str,
) -> Result<Option<UpdateManifestInfo>, UpdaterError> {
    AirGapGuard::assert_network_allowed(endpoint_url)
        .map_err(|e| UpdaterError::AirGapBlocked(e.to_string()))?;

    let resp = client
        .get(endpoint_url)
        .send()
        .await
        .map_err(|e| UpdaterError::FetchError(e.to_string()))?;

    if resp.status().as_u16() == 204 {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(UpdaterError::FetchError(format!(
            "Update server returned HTTP {}",
            resp.status()
        )));
    }

    let manifest: UpdateManifest = resp
        .json()
        .await
        .map_err(|e| UpdaterError::FetchError(format!("Invalid manifest JSON: {e}")))?;

    verify_manifest_signature(&manifest, pubkey_b64)?;

    Ok(Some(UpdateManifestInfo {
        version: manifest.version,
        body: manifest.release_notes,
        date: manifest.pub_date,
        download_url: manifest.download_url,
        signature_verified: true,
    }))
}
