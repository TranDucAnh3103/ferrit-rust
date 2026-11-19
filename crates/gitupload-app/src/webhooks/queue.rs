use super::discord::format_discord_payload;
use super::http::{send_generic_webhook, WebhookDeliveryError};
use super::slack::format_slack_payload;
use super::telegram::format_telegram_payload;
use chrono::Utc;
use gitupload_core::webhooks::{
    WebhookDeliveryRecord, WebhookEndpoint, WebhookEvent, WebhookTransport,
};
use gitupload_security::PlatformVault;
use gitupload_storage::{StoragePool, WebhooksRepository};
use rand::Rng;
use secrecy::SecretString;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

pub struct WebhookDispatcher {
    sender: mpsc::Sender<WebhookEvent>,
    client: reqwest::Client,
}

impl WebhookDispatcher {
    pub fn new(storage: StoragePool, vault: Arc<PlatformVault>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<WebhookEvent>(256);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let client_clone = client.clone();
        let worker = async move {
            info!("Webhook dispatcher background worker started");
            let webhooks_repo = WebhooksRepository::new(storage.clone());

            while let Some(event) = receiver.recv().await {
                let endpoints = match webhooks_repo.list_endpoints() {
                    Ok(eps) => eps,
                    Err(e) => {
                        error!("Failed to query webhook endpoints: {e}");
                        continue;
                    }
                };

                let event_type = event.event_type_name().to_string();
                for endpoint in endpoints {
                    if !endpoint.is_active {
                        continue;
                    }

                    // Check if endpoint is subscribed to this event or to "*"
                    let subscribed = endpoint
                        .events
                        .iter()
                        .any(|ev| ev == "*" || ev == &event_type);
                    if !subscribed {
                        continue;
                    }

                    let client = client_clone.clone();
                    let ev_clone = event.clone();
                    let ep_clone = endpoint.clone();
                    let vault_clone = vault.clone();
                    let storage_pool = storage.clone();
                    let ev_type_clone = event_type.clone();

                    // Dispatch with exponential retry
                    tokio::spawn(async move {
                        let secret: Option<SecretString> = match ep_clone.vault_secret_key.as_ref()
                        {
                            Some(key) => vault_clone.retrieve(key).await.ok(),
                            None => None,
                        };

                        let start = std::time::Instant::now();
                        let (status, attempts, err_opt) = Self::deliver_with_retry(
                            &client,
                            &ep_clone,
                            &ev_clone,
                            secret.as_ref(),
                        )
                        .await;
                        let duration_ms = start.elapsed().as_millis() as u64;

                        let delivery_record = WebhookDeliveryRecord {
                            id: uuid::Uuid::new_v4().to_string(),
                            endpoint_id: ep_clone.id.clone(),
                            event_type: ev_type_clone,
                            status_code: status,
                            attempt_count: attempts,
                            duration_ms,
                            error_message: err_opt,
                            delivered_at: Utc::now(),
                        };

                        let repo = WebhooksRepository::new(storage_pool);
                        let _ = repo.record_delivery(&delivery_record);
                    });
                }
            }
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(worker);
        } else {
            std::thread::Builder::new()
                .name("webhook-dispatcher".into())
                .spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(worker);
                    }
                })
                .ok();
        }

        Self { sender, client }
    }

    pub fn dispatch(&self, event: WebhookEvent) {
        if let Err(e) = self.sender.try_send(event) {
            warn!("Failed to enqueue webhook event: {e}");
        }
    }

    pub async fn test_ping(
        &self,
        endpoint: &WebhookEndpoint,
        secret: Option<&SecretString>,
    ) -> Result<u16, WebhookDeliveryError> {
        let dummy_event = WebhookEvent::GitPush(gitupload_core::webhooks::GitPushPayload {
            repository: "org/test-ping".into(),
            branch: "main".into(),
            commit_hash: "0000000000000000".into(),
            commit_message: "test: webhook ping verification".into(),
            author_name: "GitUpload System".into(),
            author_email: "noreply@gitupload.org".into(),
            files_changed: 1,
            timestamp: Utc::now(),
            github_url: None,
        });

        Self::deliver_single_attempt(&self.client, endpoint, &dummy_event, secret).await
    }

    pub async fn deliver_with_retry(
        client: &reqwest::Client,
        endpoint: &WebhookEndpoint,
        event: &WebhookEvent,
        secret: Option<&SecretString>,
    ) -> (Option<u16>, u32, Option<String>) {
        let max_attempts = 3;
        let mut attempt = 0;
        let mut last_err = None;

        while attempt < max_attempts {
            attempt += 1;
            match Self::deliver_single_attempt(client, endpoint, event, secret).await {
                Ok(status) => {
                    info!(
                        "Webhook delivered to {} [{}] in {} attempt(s)",
                        endpoint.name, endpoint.target_url, attempt
                    );
                    return (Some(status), attempt, None);
                }
                Err(err) => {
                    let err_msg = err.to_string();
                    warn!(
                        "Webhook delivery attempt {} to {} failed: {}",
                        attempt, endpoint.name, err_msg
                    );
                    last_err = Some(err_msg);

                    if attempt < max_attempts {
                        let base_ms = match attempt {
                            1 => 1000,
                            2 => 2000,
                            _ => 4000,
                        };
                        // Add jitter +/- 200ms
                        let jitter: i64 = {
                            let mut rng = rand::thread_rng();
                            rng.gen_range(-200..=200)
                        };
                        let sleep_ms = ((base_ms as i64) + jitter).max(100) as u64;

                        // Honor 429 Retry-After if present
                        if let WebhookDeliveryError::RateLimited(Some(duration)) = err {
                            tokio::time::sleep(duration).await;
                        } else {
                            tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                        }
                    }
                }
            }
        }

        (None, attempt, last_err)
    }

    async fn deliver_single_attempt(
        client: &reqwest::Client,
        endpoint: &WebhookEndpoint,
        event: &WebhookEvent,
        secret: Option<&SecretString>,
    ) -> Result<u16, WebhookDeliveryError> {
        match endpoint.transport {
            WebhookTransport::Discord => {
                let body = format_discord_payload(event);
                let res = client
                    .post(&endpoint.target_url)
                    .json(&body)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?;
                let status = res.status().as_u16();
                if res.status().is_success() {
                    Ok(status)
                } else {
                    let err_body = res.text().await.unwrap_or_default();
                    Err(WebhookDeliveryError::ServerError {
                        status,
                        body: err_body,
                    })
                }
            }
            WebhookTransport::Slack => {
                let body = format_slack_payload(event);
                let res = client
                    .post(&endpoint.target_url)
                    .json(&body)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?;
                let status = res.status().as_u16();
                if res.status().is_success() {
                    Ok(status)
                } else {
                    let err_body = res.text().await.unwrap_or_default();
                    Err(WebhookDeliveryError::ServerError {
                        status,
                        body: err_body,
                    })
                }
            }
            WebhookTransport::Telegram => {
                // Telegram chat_id can be in vault_secret_key or default to target_url
                let chat_id = endpoint.vault_secret_key.as_deref().unwrap_or("0");
                let body = format_telegram_payload(event, chat_id);
                let res = client
                    .post(&endpoint.target_url)
                    .json(&body)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .await?;
                let status = res.status().as_u16();
                if res.status().is_success() {
                    Ok(status)
                } else {
                    let err_body = res.text().await.unwrap_or_default();
                    Err(WebhookDeliveryError::ServerError {
                        status,
                        body: err_body,
                    })
                }
            }
            WebhookTransport::Generic => {
                let json_str = serde_json::to_string(event).unwrap_or_default();
                send_generic_webhook(
                    client,
                    &endpoint.target_url,
                    event.event_type_name(),
                    &json_str,
                    secret,
                )
                .await
            }
        }
    }
}
