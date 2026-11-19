#![forbid(unsafe_code)]

use gitupload_app::webhooks::http::send_generic_webhook;
use gitupload_core::plugin_manifest::{PluginCapabilities, PluginManifest, PluginMetadata};
use gitupload_plugins::PluginRunner;
use secrecy::SecretString;
use tempfile::tempdir;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_sprint5_e2e_integration_flow() {
    // Step 1: Initialize ephemeral git repository
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let repo = git2::Repository::init(repo_path).expect("Failed to init git repo");

    // Write a test file
    let file_path = repo_path.join("README.md");
    std::fs::write(
        &file_path,
        "# GitUpload Sprint 5 E2E\nVerified extensibility platform.",
    )
    .unwrap();

    // Stage file in index
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();

    // Step 2: Validate WASM plugin commit hook
    let plugins_dir = repo_path.join(".plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();

    // Compile a minimal WASM hook that returns 0 (success)
    let wat = r#"
        (module
            (memory (export "memory") 1)
            (func (export "plugin_on_commit_msg") (param i32 i32) (result i32)
                (i32.const 0)
            )
        )
    "#;
    let wasm_bytes = wat::parse_str(wat).expect("WAT parse failed");
    let wasm_file = plugins_dir.join("hook.wasm");
    std::fs::write(&wasm_file, wasm_bytes).unwrap();

    let manifest = PluginManifest {
        plugin: PluginMetadata {
            id: "com.stephen.e2e-hook".into(),
            name: "E2E Hook".into(),
            version: "1.0.0".into(),
            entrypoint: "hook.wasm".into(),
            author: "Stephen".into(),
            description: "E2E test plugin".into(),
            homepage: None,
        },
        capabilities: PluginCapabilities {
            commit_hook: true,
            max_memory_mb: 16,
            ..Default::default()
        },
    };

    let runner = PluginRunner::new().expect("Failed to create plugin runner");
    let hook_result = runner.run_commit_msg_hook(
        &plugins_dir,
        &manifest,
        "feat(core): complete sprint 5 e2e flow",
        "main",
        &["README.md".to_string()],
    );
    assert!(
        hook_result.is_ok(),
        "WASM hook must accept conventional commit message: {:?}",
        hook_result
    );

    // Step 3: Git Commit
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Stephen", "stephen@example.com").unwrap();
    let commit_id = repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            "feat(core): complete sprint 5 e2e flow",
            &tree,
            &[],
        )
        .expect("Git commit failed");
    assert!(!commit_id.is_zero());

    // Step 4: Dispatch Webhook Event with HMAC-SHA256 signature verification
    let mock_server = MockServer::start().await;
    let webhook_secret = "e2e_super_secret_key_999";
    let payload = serde_json::json!({
        "event": "git_push",
        "repository": "rust-gitupload",
        "branch": "main",
        "commit_sha": commit_id.to_string(),
        "summary": "feat(core): complete sprint 5 e2e flow"
    })
    .to_string();

    use ring::hmac;
    let key = hmac::Key::new(hmac::HMAC_SHA256, webhook_secret.as_bytes());
    let sig = hex::encode(hmac::sign(&key, payload.as_bytes()));
    let expected_sig = format!("sha256={}", sig);

    Mock::given(method("POST"))
        .and(path("/e2e-webhook"))
        .and(header("X-Hub-Signature-256", expected_sig.as_str()))
        .and(header("X-GitUpload-Event", "git_push"))
        .and(header_exists("X-GitUpload-Delivery"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":"delivered"}"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let secret_str = SecretString::new(webhook_secret.into());
    let delivery_result = send_generic_webhook(
        &client,
        &format!("{}/e2e-webhook", mock_server.uri()),
        "git_push",
        &payload,
        Some(&secret_str),
    )
    .await;

    assert!(
        delivery_result.is_ok(),
        "End-to-End Webhook delivery must succeed: {:?}",
        delivery_result.err()
    );
}
