use gitupload_ai::{CloudAiProvider, CloudAiSecurityEvaluator, CloudAiService, EmbeddingsClient};
use gitupload_core::models::VulnerabilitySeverity;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_security_evaluator_parses_owasp_json() {
    let sample_json = r#"```json
    [
      {
        "title": "SQL Injection via String Formatting",
        "description": "User input interpolated directly into SQL statement.",
        "severity": "Critical",
        "cwe_id": "CWE-89",
        "owasp_category": "A03:2021-Injection",
        "line_start": 12,
        "line_end": 12,
        "vulnerable_snippet": "format!(\"SELECT * FROM users WHERE id = {}\", user_id)",
        "patch_description": "Use parameterized binding",
        "replacement_code": "sqlx::query(\"SELECT * FROM users WHERE id = $1\").bind(user_id)"
      }
    ]
    ```"#;

    let findings = CloudAiSecurityEvaluator::parse_findings_json("src/db.rs", sample_json).unwrap();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, VulnerabilitySeverity::Critical);
    assert_eq!(findings[0].cwe_id.as_deref(), Some("CWE-89"));
    assert!(findings[0].suggested_fix.is_some());
}

#[tokio::test]
async fn test_embeddings_client_wiremock_roundtrip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{ "embedding": [0.25, 0.5, 0.75] }]
        })))
        .mount(&server)
        .await;

    let ai = CloudAiService::new();
    let provider = CloudAiProvider::OpenAiCompatible(server.uri());
    let key = SecretString::from("sk_test".to_string());

    let vec = EmbeddingsClient::generate_embedding(&ai, &provider, &key, "fix auth memory leak")
        .await
        .unwrap();
    assert_eq!(vec, vec![0.25_f32, 0.5_f32, 0.75_f32]);
}
