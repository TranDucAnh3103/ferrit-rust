use gitupload_ai::{
    AgentAction, AiGenerators, CloudAiProvider, CloudAiService, IntentClassifier, StaticIntent,
};
use secrecy::SecretString;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_ai_commit_and_changelog_generators_ac_p4_02() {
    let commit_server = MockServer::start().await;
    let changelog_server = MockServer::start().await;

    let commit_sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"```\\n\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"feat(auth): add keyring zeroize guard\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"\\n```\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(commit_sse),
        )
        .mount(&commit_server)
        .await;

    let changelog_sse = concat!(
        "data: {\"choices\":[{\"delta\":{\"content\":\"### 🚀 Features\\n- Add DAG visualizer\\n\"}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"### 🐛 Bug Fixes\\n- Safe sync force-with-lease\"}}]}\n\n",
        "data: [DONE]\n\n"
    );

    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(changelog_sse),
        )
        .mount(&changelog_server)
        .await;

    let service = CloudAiService::new();
    let api_key = SecretString::from("AIzaFakeGeminiTestKey".to_string());

    let generated_commit = AiGenerators::generate_commit_message(
        &service,
        &CloudAiProvider::OpenAiCompatible(commit_server.uri()),
        &api_key,
        "gemini-2.0-flash",
        "diff --git a/keyring.rs b/keyring.rs\n+SecretString::from(tok)",
    )
    .await
    .unwrap();

    assert_eq!(generated_commit, "feat(auth): add keyring zeroize guard");

    let generated_notes = AiGenerators::generate_release_changelog(
        &service,
        &CloudAiProvider::OpenAiCompatible(changelog_server.uri()),
        &api_key,
        "gemini-2.0-flash",
        "v2.2.0",
        &[
            "feat(git): add DAG visualizer".into(),
            "fix(git): safe sync force-with-lease".into(),
        ],
    )
    .await
    .unwrap();

    assert!(generated_notes.contains("### 🚀 Features"));
    assert!(generated_notes.contains("### 🐛 Bug Fixes"));

    // Verify zero-token local router classifies direct commands without network calls.
    assert_eq!(
        IntentClassifier::classify("xin chào"),
        StaticIntent::Greeting
    );
    assert_eq!(
        IntentClassifier::classify("clone https://github.com/octocat/Hello-World"),
        StaticIntent::DirectAction(AgentAction::Clone {
            url: "https://github.com/octocat/Hello-World".into(),
            dest: None,
        })
    );
}
