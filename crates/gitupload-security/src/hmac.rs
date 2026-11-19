use ring::hmac;
use secrecy::ExposeSecret;

/// Compute HMAC-SHA256 signature for a webhook payload using a secret string.
/// Returns formatted string `sha256=<hex_digest>`.
pub fn sign_payload(secret: &secrecy::SecretString, payload: &[u8]) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.expose_secret().as_bytes());
    let signature = hmac::sign(&key, payload);
    format!("sha256={}", hex::encode(signature.as_ref()))
}

/// Verify that a given hex-encoded signature matches the expected HMAC-SHA256 of payload.
/// Supports both raw hex and `sha256=<hex>` prefix.
pub fn verify_payload_signature(
    secret: &secrecy::SecretString,
    payload: &[u8],
    signature_header: &str,
) -> bool {
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.expose_secret().as_bytes());
    let sig_hex = signature_header
        .strip_prefix("sha256=")
        .unwrap_or(signature_header)
        .trim();
    let Ok(sig_bytes) = hex::decode(sig_hex) else {
        return false;
    };
    hmac::verify(&key, payload, &sig_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_hmac_sign_and_verify() {
        let secret = SecretString::new("my_secret_key_123".into());
        let payload = b"{\"event\":\"git_push\",\"branch\":\"main\"}";

        let sig = sign_payload(&secret, payload);
        assert!(sig.starts_with("sha256="));
        assert!(verify_payload_signature(&secret, payload, &sig));

        // Tampered payload fails
        assert!(!verify_payload_signature(&secret, b"tampered", &sig));

        // Wrong secret fails
        let wrong_secret = SecretString::new("other_secret".into());
        assert!(!verify_payload_signature(&wrong_secret, payload, &sig));
    }
}
