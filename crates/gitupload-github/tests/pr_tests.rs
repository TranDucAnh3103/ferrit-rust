use gitupload_github::{
    sanitize_readme_html, CreateIssueRequest, CreatePullRequest, CreateReleaseRequest,
    GitHubClient, MergePullRequest, ReviewEvent,
};
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_github_prs_issues_releases_and_xss_guard_ac_p4_04() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/repos/octo/app/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 10,
                "number": 42,
                "title": "feat(ui): primer dark theme",
                "body": "Implements Primer tokens",
                "state": "open",
                "draft": true,
                "html_url": "https://github.com/octo/app/pull/42",
                "user": { "login": "stephen" },
                "head": { "ref": "feat/primer-dark" },
                "base": { "ref": "main" },
                "mergeable": true,
                "comments": 2,
                "created_at": "2026-11-19T00:00:00Z"
            }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/repos/octo/app/pulls"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 11,
            "number": 43,
            "title": "feat(git): safe sync",
            "body": "## Summary\nAdds force-with-lease",
            "state": "open",
            "draft": false,
            "html_url": "https://github.com/octo/app/pull/43",
            "user": { "login": "stephen" },
            "head": { "ref": "feat/safe-sync" },
            "base": { "ref": "main" },
            "mergeable": true,
            "created_at": "2026-11-19T00:05:00Z"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/repos/octo/app/pulls/43/reviews"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "id": 777 })))
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/repos/octo/app/pulls/43/merge"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "merged": true })),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/repos/octo/app/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 1001,
                "number": 5,
                "title": "Bug: crash on long path",
                "body": "Long path exceeds MAX_PATH",
                "state": "open",
                "user": { "login": "alice" },
                "labels": [{ "name": "bug" }],
                "created_at": "2026-11-19T00:00:00Z"
            },
            {
                "id": 1002,
                "number": 42,
                "title": "PR item filtered out",
                "state": "open",
                "user": { "login": "stephen" },
                "labels": [],
                "pull_request": { "url": "https://api.github.com/..." },
                "created_at": "2026-11-19T00:00:00Z"
            }
        ])))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/repos/octo/app/issues"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 1003,
            "number": 6,
            "title": "feat: add release manager",
            "body": "Create releases directly from UI",
            "state": "open",
            "user": { "login": "stephen" },
            "labels": [{ "name": "enhancement" }],
            "created_at": "2026-11-19T00:06:00Z"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/repos/octo/app/releases"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": 8888,
            "html_url": "https://github.com/octo/app/releases/tag/v2.2.0"
        })))
        .mount(&server)
        .await;

    let client = GitHubClient::with_base_url(
        server.uri(),
        Some(SecretString::from("ghp_prMockToken".to_string())),
    );

    let prs = client
        .list_pull_requests("octo", "app", "open")
        .await
        .unwrap();
    assert_eq!(prs.len(), 1);
    assert!(prs[0].draft);

    let created = client
        .create_pull_request(
            "octo",
            "app",
            &CreatePullRequest {
                title: "feat(git): safe sync".into(),
                head: "feat/safe-sync".into(),
                base: "main".into(),
                body: Some("## Summary\nAdds force-with-lease".into()),
                draft: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(created.number, 43);

    let review_id = client
        .submit_pr_review("octo", "app", 43, "LGTM!", ReviewEvent::Approve)
        .await
        .unwrap();
    assert_eq!(review_id, 777);

    let merged = client
        .merge_pull_request(
            "octo",
            "app",
            43,
            &MergePullRequest {
                commit_title: Some("feat(git): safe sync (#43)".into()),
                commit_message: None,
                merge_method: "squash".into(),
            },
        )
        .await
        .unwrap();
    assert!(merged);

    // Issues list strips out Pull Requests returned by GitHub's unified /issues endpoint.
    let issues = client.list_issues("octo", "app").await.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].number, 5);

    let created_issue = client
        .create_issue(
            "octo",
            "app",
            &CreateIssueRequest {
                title: "feat: add release manager".into(),
                body: Some("Create releases directly from UI".into()),
                labels: vec!["enhancement".into()],
            },
        )
        .await
        .unwrap();
    assert_eq!(created_issue.number, 6);

    let release_id = client
        .create_release(
            "octo",
            "app",
            &CreateReleaseRequest {
                tag_name: "v2.2.0".into(),
                name: "GitUpload v2.2.0".into(),
                body: "### 🚀 Features\n- Primer Desktop UI".into(),
                draft: false,
                prerelease: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(release_id, 8888);

    // SEC-05: XSS payloads in rendered HTML must be neutralized.
    let dirty_html = r#"<h1>Hello</h1><img src="x" onerror="alert(1)"><script>steal()</script><a href="javascript:alert(2)">click</a>"#;
    let safe_html = sanitize_readme_html(dirty_html);
    assert!(!safe_html.contains("<script"));
    assert!(!safe_html.contains("onerror"));
    assert!(!safe_html.contains("javascript:"));
    assert!(safe_html.contains("<h1>Hello</h1>"));
}
