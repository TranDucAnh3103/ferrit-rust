//! Pure-Rust SSH cryptographic key generation (Ed25519 & RSA 4096) and OpenSSH formatting.

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use chrono::Utc;
use ed25519_dalek::{Signer as _, SigningKey as Ed25519SigningKey};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{SshKeyAlgorithm, SshKeyDescriptor};
use rand::rngs::OsRng;
use rsa::pkcs1v15::SigningKey as RsaSigningKey;
use rsa::signature::{RandomizedSigner as _, SignatureEncoding as _};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{BigUint, RsaPrivateKey};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Generated SSH keypair containing non-sensitive metadata and zeroized serialized private material.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct GeneratedSshKeypair {
    /// Non-sensitive metadata descriptor safe for SQLite and IPC.
    #[zeroize(skip)]
    pub descriptor: SshKeyDescriptor,
    /// OpenSSH wire-format public key blob (used inside SSH Agent protocol).
    #[zeroize(skip)]
    pub public_key_blob: Vec<u8>,
    /// Secret JSON-encoded key material stored solely inside OS Credential Vault.
    pub private_material_json: String,
}

/// Internal serialized representation stored encrypted inside Windows Credential Manager.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(tag = "alg", rename_all = "snake_case")]
pub(crate) enum StoredPrivateMaterial {
    Ed25519 {
        seed_b64: String,
    },
    Rsa4096 {
        n_b64: String,
        e_b64: String,
        d_b64: String,
        primes_b64: Vec<String>,
    },
}

/// Writes an SSH length-prefixed binary string (`uint32` big-endian length + bytes).
pub(crate) fn push_ssh_string(buf: &mut Vec<u8>, data: &[u8]) {
    let len = data.len() as u32;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(data);
}

/// Writes an SSH `mpint` (RFC 4251 Section 5) from big-endian unsigned integer bytes.
fn push_ssh_mpint(buf: &mut Vec<u8>, unsigned_be: &[u8]) {
    let trimmed = unsigned_be
        .iter()
        .position(|&b| b != 0)
        .map(|idx| &unsigned_be[idx..])
        .unwrap_or(&[]);
    if trimmed.is_empty() {
        buf.extend_from_slice(&0u32.to_be_bytes());
        return;
    }
    if trimmed[0] & 0x80 != 0 {
        let len = (trimmed.len() + 1) as u32;
        buf.extend_from_slice(&len.to_be_bytes());
        buf.push(0x00);
        buf.extend_from_slice(trimmed);
    } else {
        push_ssh_string(buf, trimmed);
    }
}

/// Computes `SHA256:<base64_no_pad>` OpenSSH fingerprint from an SSH public key wire blob.
pub fn compute_fingerprint_sha256(public_key_blob: &[u8]) -> String {
    let digest = Sha256::digest(public_key_blob);
    format!("SHA256:{}", STANDARD_NO_PAD.encode(digest))
}

/// Generates a fresh cryptographic SSH keypair (`Ed25519` or `RSA 4096`) in pure Rust.
pub fn generate_ssh_keypair(
    title: impl Into<String>,
    algorithm: SshKeyAlgorithm,
) -> GitUploadResult<GeneratedSshKeypair> {
    let clean_title = title.into().trim().to_string();
    if clean_title.is_empty() {
        return Err(GitUploadError::Config(
            "SSH key title cannot be empty".to_string(),
        ));
    }

    match algorithm {
        SshKeyAlgorithm::Ed25519 => generate_ed25519(&clean_title),
        SshKeyAlgorithm::Rsa4096 => generate_rsa(&clean_title, 4096),
    }
}

/// Generates an RSA SSH keypair with configurable modulus bit-length (4096 in production, 2048 available for fast tests).
pub(crate) fn generate_rsa(title: &str, bits: usize) -> GitUploadResult<GeneratedSshKeypair> {
    let mut rng = OsRng;
    let priv_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| GitUploadError::Vault(format!("RSA-{bits} generation failed: {e}")))?;
    let pub_key = priv_key.to_public_key();

    let e_bytes = pub_key.e().to_bytes_be();
    let n_bytes = pub_key.n().to_bytes_be();
    let mut blob = Vec::new();
    push_ssh_string(&mut blob, b"ssh-rsa");
    push_ssh_mpint(&mut blob, &e_bytes);
    push_ssh_mpint(&mut blob, &n_bytes);

    let fingerprint = compute_fingerprint_sha256(&blob);
    let public_key_line = format!("ssh-rsa {} {}", STANDARD.encode(&blob), title);

    let material = StoredPrivateMaterial::Rsa4096 {
        n_b64: STANDARD.encode(&n_bytes),
        e_b64: STANDARD.encode(&e_bytes),
        d_b64: STANDARD.encode(priv_key.d().to_bytes_be()),
        primes_b64: priv_key
            .primes()
            .iter()
            .map(|p| STANDARD.encode(p.to_bytes_be()))
            .collect(),
    };

    let private_material_json = serde_json::to_string(&material)
        .map_err(|e| GitUploadError::Vault(format!("Failed to serialize RSA key: {e}")))?;

    Ok(GeneratedSshKeypair {
        descriptor: SshKeyDescriptor {
            key_id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            algorithm: SshKeyAlgorithm::Rsa4096,
            public_key: public_key_line,
            fingerprint_sha256: fingerprint,
            created_at: Utc::now(),
            is_registered_on_github: false,
        },
        public_key_blob: blob,
        private_material_json,
    })
}

fn generate_ed25519(title: &str) -> GitUploadResult<GeneratedSshKeypair> {
    let mut rng = OsRng;
    let signing_key = Ed25519SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    let mut blob = Vec::new();
    push_ssh_string(&mut blob, b"ssh-ed25519");
    push_ssh_string(&mut blob, verifying_key.as_bytes());

    let fingerprint = compute_fingerprint_sha256(&blob);
    let public_key_line = format!("ssh-ed25519 {} {}", STANDARD.encode(&blob), title);

    let mut seed_bytes = Zeroizing::new(signing_key.to_bytes());
    let seed_b64 = STANDARD.encode(*seed_bytes);
    seed_bytes.zeroize();

    let material = StoredPrivateMaterial::Ed25519 { seed_b64 };
    let private_material_json = serde_json::to_string(&material)
        .map_err(|e| GitUploadError::Vault(format!("Failed to serialize Ed25519 key: {e}")))?;

    Ok(GeneratedSshKeypair {
        descriptor: SshKeyDescriptor {
            key_id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            algorithm: SshKeyAlgorithm::Ed25519,
            public_key: public_key_line,
            fingerprint_sha256: fingerprint,
            created_at: Utc::now(),
            is_registered_on_github: false,
        },
        public_key_blob: blob,
        private_material_json,
    })
}

/// Parses an OpenSSH public key line (`ssh-ed25519 AAAA... comment`) into its raw wire blob.
pub fn decode_openssh_pubkey_blob(public_key_line: &str) -> GitUploadResult<Vec<u8>> {
    let mut parts = public_key_line.split_whitespace();
    let _alg = parts.next().ok_or_else(|| {
        GitUploadError::Config("Malformed OpenSSH public key (missing algorithm)".to_string())
    })?;
    let b64 = parts.next().ok_or_else(|| {
        GitUploadError::Config("Malformed OpenSSH public key (missing base64 blob)".to_string())
    })?;
    STANDARD
        .decode(b64)
        .map_err(|e| GitUploadError::Vault(format!("Invalid base64 public key blob: {e}")))
}

/// Signs arbitrary challenge bytes using serialized vault key material and returns an OpenSSH SSH2 signature blob.
pub(crate) fn sign_challenge_with_secret(
    secret_json: &SecretString,
    challenge: &[u8],
    flags: u32,
) -> GitUploadResult<Vec<u8>> {
    let material: StoredPrivateMaterial = serde_json::from_str(secret_json.expose_secret())
        .map_err(|e| GitUploadError::Vault(format!("Invalid stored SSH key material: {e}")))?;

    match &material {
        StoredPrivateMaterial::Ed25519 { seed_b64 } => {
            let mut seed_vec = Zeroizing::new(
                STANDARD
                    .decode(seed_b64)
                    .map_err(|e| GitUploadError::Vault(format!("Invalid Ed25519 seed: {e}")))?,
            );
            let seed_arr: [u8; 32] = seed_vec.as_slice().try_into().map_err(|_| {
                GitUploadError::Vault("Invalid Ed25519 seed length (expected 32 bytes)".to_string())
            })?;
            let signing_key = Ed25519SigningKey::from_bytes(&seed_arr);
            seed_vec.zeroize();
            let signature = signing_key.sign(challenge);

            let mut sig_blob = Vec::new();
            push_ssh_string(&mut sig_blob, b"ssh-ed25519");
            push_ssh_string(&mut sig_blob, &signature.to_bytes());
            Ok(sig_blob)
        }
        StoredPrivateMaterial::Rsa4096 {
            n_b64,
            e_b64,
            d_b64,
            primes_b64,
        } => {
            let n = BigUint::from_bytes_be(
                &STANDARD
                    .decode(n_b64)
                    .map_err(|e| GitUploadError::Vault(format!("Invalid RSA modulus: {e}")))?,
            );
            let e = BigUint::from_bytes_be(
                &STANDARD
                    .decode(e_b64)
                    .map_err(|e| GitUploadError::Vault(format!("Invalid RSA exponent: {e}")))?,
            );
            let d = BigUint::from_bytes_be(&STANDARD.decode(d_b64).map_err(|e| {
                GitUploadError::Vault(format!("Invalid RSA private exponent: {e}"))
            })?);
            let mut primes = Vec::with_capacity(primes_b64.len());
            for p_b64 in primes_b64 {
                primes.push(BigUint::from_bytes_be(&STANDARD.decode(p_b64).map_err(
                    |e| GitUploadError::Vault(format!("Invalid RSA prime: {e}")),
                )?));
            }
            let priv_key = RsaPrivateKey::from_components(n, e, d, primes).map_err(|err| {
                GitUploadError::Vault(format!("Invalid RSA key components: {err}"))
            })?;

            let mut rng = OsRng;
            // RFC 8332 flags: 0x04 -> rsa-sha2-512, 0x02 or default -> rsa-sha2-256
            let (alg_name, sig_bytes) = if flags & 0x04 != 0 {
                let signer = RsaSigningKey::<Sha512>::new(priv_key);
                (
                    b"rsa-sha2-512".as_slice(),
                    signer.sign_with_rng(&mut rng, challenge).to_vec(),
                )
            } else {
                let signer = RsaSigningKey::<Sha256>::new(priv_key);
                (
                    b"rsa-sha2-256".as_slice(),
                    signer.sign_with_rng(&mut rng, challenge).to_vec(),
                )
            };

            let mut sig_blob = Vec::new();
            push_ssh_string(&mut sig_blob, alg_name);
            push_ssh_string(&mut sig_blob, &sig_bytes);
            Ok(sig_blob)
        }
    }
}
