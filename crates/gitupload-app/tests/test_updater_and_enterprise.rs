use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use gitupload_app::platform::{atomic_swap_at, EnterprisePolicyManager};
use gitupload_app::updater_channel::{
    fetch_and_verify_manifest, verify_ed25519_payload, verify_manifest_signature, UpdateChannel,
    UpdateManifest, UpdaterError,
};
use rand::rngs::OsRng;
use tempfile::tempdir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_channel_routing_urls() {
    assert_eq!(
        UpdateChannel::Stable.endpoint_url("2.5.0", "windows"),
        "https://releases.gitupload.org/api/v1/update/windows/2.5.0?channel=stable"
    );
    assert_eq!(
        UpdateChannel::Beta.endpoint_url("2.5.0", "macos"),
        "https://releases.gitupload.org/api/v1/update/macos/2.5.0?channel=beta"
    );
    assert_eq!(
        UpdateChannel::Nightly.endpoint_url("2.5.0", "linux"),
        "https://releases.gitupload.org/api/v1/update/linux/2.5.0?channel=nightly"
    );
}

#[tokio::test]
async fn test_updater_signature_verification_and_tamper_rejection() {
    let mut rng = OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let pubkey_b64 = STANDARD.encode(signing_key.verifying_key().as_bytes());

    let version = "2.5.0";
    let sha = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let dl = "http://127.0.0.1/gitupload-2.5.0.msi";
    let canonical = format!("{version}|{sha}|{dl}");
    let sig_b64 = STANDARD.encode(signing_key.sign(canonical.as_bytes()).to_bytes());

    let valid_manifest = UpdateManifest {
        version: version.into(),
        release_notes: "Enterprise Sprint 4".into(),
        pub_date: "2026-11-19T00:00:00Z".into(),
        signature: sig_b64,
        download_url: dl.into(),
        sha256_checksum: sha.into(),
    };
    assert!(verify_manifest_signature(&valid_manifest, &pubkey_b64).is_ok());

    // Tampered version must reject with SignatureVerificationFailed
    let mut tampered = valid_manifest.clone();
    tampered.version = "2.5.1".into();
    let err = verify_manifest_signature(&tampered, &pubkey_b64).unwrap_err();
    assert!(matches!(err, UpdaterError::SignatureVerificationFailed(_)));

    // Invalid payload signature must fail
    let raw_err =
        verify_ed25519_payload(&pubkey_b64, b"wrong", &valid_manifest.signature).unwrap_err();
    assert!(matches!(
        raw_err,
        UpdaterError::SignatureVerificationFailed(_)
    ));

    // Wiremock manifest endpoint integration test
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/update"))
        .and(query_param("channel", "beta"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&valid_manifest))
        .mount(&server)
        .await;

    let endpoint = format!("{}/update?channel=beta", server.uri());
    let client = reqwest::Client::new();
    let info = fetch_and_verify_manifest(&client, &endpoint, &pubkey_b64)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(info.version, "2.5.0");
    assert!(info.signature_verified);
}

#[test]
fn test_windows_atomic_swap_and_enterprise_policy_loader() {
    let dir = tempdir().unwrap();
    let current = dir.path().join("gitupload-app.exe");
    let next = dir.path().join("gitupload-app-new.exe");
    std::fs::write(&current, b"OLD_BINARY").unwrap();
    std::fs::write(&next, b"NEW_BINARY").unwrap();

    atomic_swap_at(&current, &next).unwrap();
    assert_eq!(std::fs::read(&current).unwrap(), b"NEW_BINARY");

    let policy_json = dir.path().join("policy.json");
    std::fs::write(
        &policy_json,
        r#"{
          "enterprise": {
            "organization_name": "Corp IT",
            "strict_airgap": false,
            "disable_cloud_ai": true,
            "disable_telemetry": true
          }
        }"#,
    )
    .unwrap();
    let loaded = EnterprisePolicyManager::load_effective_policy(Some(&policy_json));
    assert_eq!(loaded.organization_name, "Corp IT");
    assert!(loaded.gpo_locks_cloud_ai);
}
