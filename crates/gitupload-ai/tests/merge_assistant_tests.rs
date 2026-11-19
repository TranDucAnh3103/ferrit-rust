use gitupload_ai::{CloudAiProvider, CloudAiService, MergeAssistant};
use gitupload_core::models::ConflictHunk;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_merge_assistant_strips_fences_via_mock_sse() {
    let server = MockServer::start().await;
    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"```rust\\n\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"let port = 8443;\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"\\n```\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&server)
        .await;

    let provider =
        CloudAiProvider::OpenAiCompatible(format!("{}/v1/chat/completions", server.uri()));
    let service = CloudAiService::new();
    let api_key = SecretString::from("sk_test_key".to_string());
    let hunk = ConflictHunk {
        hunk_id: "abc:0".to_string(),
        file_path: "src/config.rs".to_string(),
        hunk_index: 0,
        start_line_ours: 1,
        end_line_ours: 2,
        start_line_theirs: 4,
        end_line_theirs: 5,
        content_ours: "let port = 8080;".to_string(),
        content_theirs: "let port = 8443;".to_string(),
        content_base: Some("let port = 3000;".to_string()),
    };

    let resolved =
        MergeAssistant::auto_resolve_conflict(&service, &provider, &api_key, "gpt-4o", &hunk)
            .await
            .unwrap();
    assert_eq!(resolved, "let port = 8443;");
}
