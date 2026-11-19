use crate::sandbox::PluginError;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// Verifies that a plugin bundle has the expected SHA256 checksum and a valid Ed25519 publisher signature.
pub fn verify_plugin_bundle(
    bundle_bytes: &[u8],
    expected_sha256: &str,
    signature_hex: &str,
    publisher_pubkey_hex: &str,
) -> Result<(), PluginError> {
    // 1. Verify SHA256 Checksum
    let mut hasher = Sha256::new();
    hasher.update(bundle_bytes);
    let calculated_sha256 = hex::encode(hasher.finalize());

    if !calculated_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(PluginError::Verification(format!(
            "SHA256 checksum mismatch: expected {}, calculated {}",
            expected_sha256, calculated_sha256
        )));
    }

    // 2. Decode Public Key & Signature
    let pubkey_bytes = hex::decode(publisher_pubkey_hex)
        .map_err(|e| PluginError::Verification(format!("Invalid publisher public key hex: {e}")))?;
    let pubkey_slice: [u8; 32] = pubkey_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PluginError::Verification("Publisher public key must be 32 bytes".into()))?;

    let verifying_key = VerifyingKey::from_bytes(&pubkey_slice)
        .map_err(|e| PluginError::Verification(format!("Invalid Ed25519 public key: {e}")))?;

    let sig_bytes = hex::decode(signature_hex)
        .map_err(|e| PluginError::Verification(format!("Invalid signature hex: {e}")))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| PluginError::Verification(format!("Invalid signature structure: {e}")))?;

    // 3. Cryptographic Verification
    verifying_key
        .verify_strict(bundle_bytes, &signature)
        .map_err(|e| {
            PluginError::Verification(format!("Ed25519 signature verification failed: {e}"))
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_verify_bundle_success() {
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let bundle_bytes = b"fake-tar-gz-plugin-content";
        let mut hasher = Sha256::new();
        hasher.update(bundle_bytes);
        let sha256_hex = hex::encode(hasher.finalize());

        use ed25519_dalek::Signer;
        let signature = signing_key.sign(bundle_bytes);
        let sig_hex = hex::encode(signature.to_bytes());
        let pubkey_hex = hex::encode(verifying_key.as_bytes());

        let res = verify_plugin_bundle(bundle_bytes, &sha256_hex, &sig_hex, &pubkey_hex);
        assert!(res.is_ok());

        // Altering sha256 fails
        assert!(verify_plugin_bundle(bundle_bytes, "000000", &sig_hex, &pubkey_hex).is_err());

        // Altering bundle content fails signature check
        assert!(
            verify_plugin_bundle(b"altered content", &sha256_hex, &sig_hex, &pubkey_hex).is_err()
        );
    }
}
