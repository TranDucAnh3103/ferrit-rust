use gitupload_core::models::{BranchMergeStatus, StaleBranchInfo};
use gitupload_github::{GitHubClient, SquashReconciler};
use secrecy::SecretString;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_squash_reconciler_upgrades_merged_pr_status() {
    let server = MockServer::start().await;
    let client = GitHubClient::with_base_url(
        server.uri(),
        Some(SecretString::from("ghp_mock".to_string())),
    );

    Mock::given(method("GET"))
        .and(path("/repos/acme/demo/pulls"))
        .and(query_param("head", "acme:feat/squashed"))
        .and(query_param("state", "closed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "number": 42,
                "merged_at": "2026-11-18T12:00:00Z"
            }
        ])))
        .mount(&server)
        .await;

    let mut candidates = vec![
        StaleBranchInfo {
            name: "feat/squashed".into(),
            is_remote: false,
            remote_name: None,
            last_commit_sha: "abc".into(),
            last_commit_date: 0,
            days_since_last_commit: 3,
            author_name: "Dev".into(),
            merge_status: BranchMergeStatus::UnmergedAhead,
            is_protected: false,
            associated_pr_number: None,
        },
        StaleBranchInfo {
            name: "release/v1.0".into(),
            is_remote: false,
            remote_name: None,
            last_commit_sha: "def".into(),
            last_commit_date: 0,
            days_since_last_commit: 120,
            author_name: "Dev".into(),
            merge_status: BranchMergeStatus::StaleInactive,
            is_protected: true,
            associated_pr_number: None,
        },
    ];

    SquashReconciler::reconcile_squashed_branches(&client, "acme", "demo", &mut candidates)
        .await
        .unwrap();

    assert_eq!(
        candidates[0].merge_status,
        BranchMergeStatus::SquashedAndMerged
    );
    assert_eq!(candidates[0].associated_pr_number, Some(42));
    assert_eq!(candidates[1].merge_status, BranchMergeStatus::StaleInactive);
}
