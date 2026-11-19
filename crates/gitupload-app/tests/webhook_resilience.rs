#![forbid(unsafe_code)]

use gitupload_app::webhooks::http::{send_generic_webhook, WebhookDeliveryError};
use secrecy::SecretString;
use std::time::Duration;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_webhook_retries_on_500_server_error() {
    let mock_server = MockServer::start().await;

    // Fail first 2 attempts with 500, succeed on 3rd attempt
    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .up_to_n_times(2)
        .expect(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/webhook"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = r#"{"event":"git_push","branch":"main"}"#;
    let secret = SecretString::new("test_secret_key_123".into());

    let mut attempts = 0;
    let mut success = false;

    for delay in [Duration::from_millis(50), Duration::from_millis(100)] {
        attempts += 1;
        let res = send_generic_webhook(
            &client,
            &format!("{}/webhook", mock_server.uri()),
            "git_push",
            payload,
            Some(&secret),
        )
        .await;

        if res.is_ok() {
            success = true;
            break;
        }
        tokio::time::sleep(delay).await;
    }

    if !success {
        attempts += 1;
        let final_res = send_generic_webhook(
            &client,
            &format!("{}/webhook", mock_server.uri()),
            "git_push",
            payload,
            Some(&secret),
        )
        .await;
        assert!(final_res.is_ok(), "3rd attempt must succeed");
    }

    assert_eq!(attempts, 3, "Expected exactly 3 delivery attempts");
}

#[tokio::test]
async fn test_webhook_hmac_signature_correctness() {
    let mock_server = MockServer::start().await;
    let payload = r#"{"action":"ping"}"#;
    let secret = "my_vault_webhook_secret";

    // Expected HMAC-SHA256 digest
    use ring::hmac;
    let key = hmac::Key::new(hmac::HMAC_SHA256, secret.as_bytes());
    let sig = hex::encode(hmac::sign(&key, payload.as_bytes()));
    let expected_sig_header = format!("sha256={}", sig);

    Mock::given(method("POST"))
        .and(path("/verify-hmac"))
        .and(header("X-Hub-Signature-256", expected_sig_header.as_str()))
        .and(header("X-GitUpload-Event", "ping"))
        .and(header_exists("X-GitUpload-Delivery"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"status":"verified"}"#))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let secret_str = SecretString::new(secret.into());
    let res = send_generic_webhook(
        &client,
        &format!("{}/verify-hmac", mock_server.uri()),
        "ping",
        payload,
        Some(&secret_str),
    )
    .await;

    assert!(
        res.is_ok(),
        "Webhook delivery with valid HMAC signature must succeed: {:?}",
        res.err()
    );
}

#[tokio::test]
async fn test_webhook_rate_limit_429_handling() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/rate-limited"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "2")
                .set_body_string("Rate limit exceeded"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let payload = r#"{"test":true}"#;

    let res = send_generic_webhook(
        &client,
        &format!("{}/rate-limited", mock_server.uri()),
        "test_event",
        payload,
        None,
    )
    .await;

    match res {
        Err(WebhookDeliveryError::RateLimited(Some(duration))) => {
            assert_eq!(
                duration,
                Duration::from_secs(2),
                "Should parse Retry-After header of 2 seconds"
            );
        }
        other => panic!("Expected RateLimited error, got {:?}", other),
    }
}
