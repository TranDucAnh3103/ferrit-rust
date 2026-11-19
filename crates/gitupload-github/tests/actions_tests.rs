use gitupload_core::errors::GitHubError;
use gitupload_github::GitHubClient;
use secrecy::SecretString;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_github_actions_monitor_and_control_ac_p4_03() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/octo/gitupload/actions/runs"))
        .and(header("authorization", "Bearer ghp_actionsMockToken123456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "workflow_runs": [
                {
                    "id": 99001,
                    "name": "CI Build & Clippy",
                    "head_branch": "main",
                    "head_sha": "abc1234",
                    "event": "push",
                    "status": "completed",
                    "conclusion": "failure",
                    "html_url": "https://github.com/octo/gitupload/actions/runs/99001",
                    "created_at": "2026-11-19T00:00:00Z"
                }
            ]
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/repos/octo/gitupload/actions/runs/99001/rerun-failed-jobs",
        ))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/repos/octo/gitupload/actions/runs/99001/cancel"))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(
            "/repos/octo/gitupload/actions/runs/401/rerun-failed-jobs",
        ))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string("Unauthorized token ghp_actionsMockToken123456"),
        )
        .mount(&server)
        .await;

    let client = GitHubClient::with_base_url(
        server.uri(),
        Some(SecretString::from("ghp_actionsMockToken123456".to_string())),
    );

    let runs = client
        .list_workflow_runs("octo", "gitupload")
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, 99001);
    assert_eq!(runs[0].conclusion.as_deref(), Some("failure"));

    client
        .rerun_failed_jobs("octo", "gitupload", 99001)
        .await
        .unwrap();

    client
        .cancel_workflow_run("octo", "gitupload", 99001)
        .await
        .unwrap();

    // SEC-02: Verify HTTP error messages sanitize leaked PAT patterns.
    let err = client
        .rerun_failed_jobs("octo", "gitupload", 401)
        .await
        .unwrap_err();
    assert!(matches!(err, GitHubError::Unauthorized(_)));
    assert!(!err.to_string().contains("ghp_actionsMockToken123456"));
}
