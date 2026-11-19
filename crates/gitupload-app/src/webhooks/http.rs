use gitupload_security::hmac::sign_payload;
use secrecy::SecretString;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebhookDeliveryError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Server returned error code {status}: {body}")]
    ServerError { status: u16, body: String },
    #[error("Timeout occurred during delivery")]
    Timeout,
    #[error("Rate limited (429), retry after {0:?}")]
    RateLimited(Option<Duration>),
}

pub async fn send_generic_webhook(
    client: &reqwest::Client,
    url: &str,
    event_type: &str,
    payload_json: &str,
    secret: Option<&SecretString>,
) -> Result<u16, WebhookDeliveryError> {
    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "GitUpload-Dispatcher/2.6")
        .header("X-GitUpload-Event", event_type)
        .header("X-GitUpload-Delivery", uuid::Uuid::new_v4().to_string());

    if let Some(sec) = secret {
        let hex_sig = sign_payload(sec, payload_json.as_bytes());
        req = req.header("X-Hub-Signature-256", hex_sig);
    }

    let res = req
        .body(payload_json.to_string())
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    let status = res.status().as_u16();
    if res.status().is_success() {
        Ok(status)
    } else if status == 429 {
        let retry_after = res
            .headers()
            .get("Retry-After")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);
        Err(WebhookDeliveryError::RateLimited(retry_after))
    } else {
        let body = res.text().await.unwrap_or_default();
        Err(WebhookDeliveryError::ServerError { status, body })
    }
}
