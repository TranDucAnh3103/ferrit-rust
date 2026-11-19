use gitupload_github::CorporateProxyConfig;
use tempfile::tempdir;

#[test]
fn test_custom_ca_cert_pem_rejection_and_proxy_builder() {
    let cfg = CorporateProxyConfig {
        proxy_url: "http://proxy.acme.internal:8080".to_string(),
        custom_ca_cert_path: None,
        bypass_hosts: vec!["localhost".to_string(), "*.internal".to_string()],
    };
    assert!(cfg.build_http_client().is_ok());

    let dir = tempdir().unwrap();
    let bad_pem = dir.path().join("bad_ca.pem");
    std::fs::write(&bad_pem, b"NOT A VALID CERTIFICATE").unwrap();

    let cfg_bad = CorporateProxyConfig {
        proxy_url: String::new(),
        custom_ca_cert_path: Some(bad_pem.to_string_lossy().to_string()),
        bypass_hosts: Vec::new(),
    };
    assert!(cfg_bad.build_http_client().is_err());
}
