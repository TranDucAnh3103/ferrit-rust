use gitupload_core::errors::GitHubError;
use gitupload_github::GitHubClient;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_github_register_ssh_key_and_422_duplicate() {
    let server = MockServer::start().await;
    let client = GitHubClient::with_base_url(
        server.uri(),
        Some(SecretString::from("ghp_mock_token_123".to_string())),
    );

    Mock::given(method("POST"))
        .and(path("/user/keys"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 99123,
            "key": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockKeyData test",
            "title": "GitUpload Desktop",
            "url": "https://api.github.com/user/keys/99123",
            "created_at": "2026-11-19T00:00:00Z"
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let created = client
        .register_ssh_key(
            "GitUpload Desktop",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockKeyData test",
        )
        .await
        .unwrap();
    assert_eq!(created.id, 99123);

    Mock::given(method("POST"))
        .and(path("/user/keys"))
        .respond_with(ResponseTemplate::new(422).set_body_json(serde_json::json!({
            "message": "Validation Failed",
            "errors": [{ "message": "key is already in use" }]
        })))
        .mount(&server)
        .await;

    let dup_err = client
        .register_ssh_key(
            "GitUpload Desktop",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockKeyData test",
        )
        .await
        .unwrap_err();
    match dup_err {
        GitHubError::Api { status, message } => {
            assert_eq!(status, 422);
            assert!(message.contains("already registered"));
        }
        other => panic!("Expected 422 GitHubError::Api, got {other:?}"),
    }
}
