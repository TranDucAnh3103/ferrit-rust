use gitupload_core::{AirGapGuard, EnterprisePolicyFile};
use tempfile::tempdir;

#[test]
fn test_airgap_blocks_public_internet_and_allows_local_intranet() {
    AirGapGuard::enable();

    // Public internet must be intercepted before socket creation
    assert!(AirGapGuard::assert_network_allowed("https://api.github.com/user").is_err());
    assert!(AirGapGuard::assert_network_allowed("https://api.openai.com/v1/chat").is_err());

    // Localhost & enterprise intranet hosts must succeed
    assert!(AirGapGuard::assert_network_allowed("http://127.0.0.1:8080/api").is_ok());
    assert!(AirGapGuard::assert_network_allowed("http://localhost:5173").is_ok());
    assert!(AirGapGuard::assert_network_allowed("https://git.acme.internal/org/repo").is_ok());
    assert!(AirGapGuard::assert_network_allowed("https://10.20.30.40/gitea").is_ok());

    AirGapGuard::disable();
    assert!(AirGapGuard::assert_network_allowed("https://api.github.com/user").is_ok());
}

#[test]
fn test_enterprise_policy_file_loading_and_remote_allowlist() {
    let dir = tempdir().unwrap();
    let policy_path = dir.path().join("policy.json");
    std::fs::write(
        &policy_path,
        r#"{
          "enterprise": {
            "organization_name": "Acme Defense Corp",
            "strict_airgap": true,
            "disable_telemetry": true,
            "allowed_remotes": ["https://git.acme.internal/*"],
            "enforced_commit_sign": true
          }
        }"#,
    )
    .unwrap();

    let policy = EnterprisePolicyFile::load_from_path(&policy_path)
        .unwrap()
        .unwrap();
    assert_eq!(policy.enterprise.organization_name, "Acme Defense Corp");
    assert!(policy.enterprise.strict_airgap);
    assert!(policy.is_remote_allowed("https://git.acme.internal/core/app.git"));
    assert!(!policy.is_remote_allowed("https://github.com/public/leak.git"));
}
