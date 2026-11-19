use gitupload_ai::{ChatMessage, CloudAiProvider, CloudAiService};
use secrecy::SecretString;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_cloud_ai_sse_streaming_and_failover_ac_p4_01() {
    let primary_fail_server = MockServer::start().await;
    let backup_sse_server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limit ghp_leakyToken"))
        .mount(&primary_fail_server)
        .await;

    let sse_body = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"feat\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"(ai): \"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"streaming ok\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body),
        )
        .mount(&backup_sse_server)
        .await;

    let service = CloudAiService::new();
    let api_key = SecretString::from("sk-test-key-12345".to_string());
    let messages = vec![ChatMessage {
        role: "user".into(),
        content: "Summarize git diff".into(),
    }];

    let mut chunks = Vec::new();
    let full = service
        .stream_completion(
            &CloudAiProvider::OpenAiCompatible(backup_sse_server.uri()),
            &api_key,
            "gpt-4o",
            &messages,
            |chunk| chunks.push(chunk.to_string()),
        )
        .await
        .unwrap();

    assert_eq!(chunks, vec!["feat", "(ai): ", "streaming ok"]);
    assert_eq!(full, "feat(ai): streaming ok");

    // Verify failover from 429 rate-limited primary endpoint to healthy secondary endpoint.
    let candidates = vec![
        (
            CloudAiProvider::OpenAiCompatible(primary_fail_server.uri()),
            api_key.clone(),
            "model-primary".to_string(),
        ),
        (
            CloudAiProvider::OpenAiCompatible(backup_sse_server.uri()),
            api_key,
            "model-backup".to_string(),
        ),
    ];

    let mut failover_chunks = Vec::new();
    let recovered = service
        .complete_with_failover(&candidates, &messages, |c| {
            failover_chunks.push(c.to_string())
        })
        .await
        .unwrap();

    assert_eq!(recovered, "feat(ai): streaming ok");
    assert_eq!(failover_chunks.len(), 3);
}
