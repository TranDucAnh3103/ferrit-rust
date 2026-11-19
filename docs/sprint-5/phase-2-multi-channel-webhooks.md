# Sprint 5 Phase 2 — Multi-Channel Event Webhook Dispatcher
**Document Identifier:** SPRINT5-PHASE2-SPEC  
**Target Milestone:** Release v2.6.0-extensible  
**Status:** Approved Technical Architecture  
**Parent Sprint Guide:** [Sprint 5 Master Guide](file:///F:/Build/JavaToolGitAutoUpload/rust-gitupload/docs/sprint-5/README.md)  

---

## 1. Overview & Architectural Objectives

Phase 2 introduces a high-reliability, asynchronous **Multi-Channel Webhook Dispatcher** to **GitUpload**. Enterprise developer teams frequently rely on automated notifications in team channels when code is pushed, syncs fail, or pull requests are authored.

### 1.1 Architectural Pillars
1. **Multi-Platform Support**: Native formatting payloads for **Discord** (Embeds), **Slack** (Block Kit), **Telegram** (Bot API / MarkdownV2), and **Generic HTTP** endpoints.
2. **Cryptographic Payload Signing**: Generic HTTP webhooks calculate standard HMAC-SHA256 digests (`X-Hub-Signature-256`), with pre-shared secrets fetched securely from the **OS Credential Vault**.
3. **Asynchronous Non-Blocking Pipeline**: Webhook delivery runs inside background Tokio worker tasks decoupled from UI threads and local Git operations via `tokio::sync::mpsc`.
4. **Resilient Retry & Dead-Letter Auditing**: Transient HTTP errors (5xx, timeouts, 429 rate limits) trigger exponential backoff with jitter (1s, 2s, 4s). Delivery results are persisted to SQLite for full auditability.

```
+------------------------------------------------------------------------------------------+
|                             Multi-Channel Webhook Pipeline                               |
|                                                                                          |
|   +----------------------------------------------------------------------------------+   |
|   | GitUpload Event Originators (Push, Sync, PR, Batch, Hook)                        |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Event Emitted                             |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Dispatcher Queue (tokio::sync::mpsc::channel(256))                               |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Dequeue                                   |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Formatters & Transports                                                          |   |
|   |                                                                                  |   |
|   |  +--------------------+  +--------------------+  +----------------------------+  |   |
|   |  | Discord Embedder   |  | Slack Block Kit    |  | Telegram MarkdownV2       |  |   |
|   |  +--------------------+  +--------------------+  +----------------------------+  |   |
|   |  | Generic HTTP Dispatcher (HMAC-SHA256 Signing via OS Vault Secret)          |  |   |
|   |  +----------------------------------------------------------------------------+  |   |
|   +------------------------------------------+---------------------------------------+   |
|                                              | Delivery Attempt                          |
|                                              v                                           |
|   +----------------------------------------------------------------------------------+   |
|   | Resilience Engine (Exponential Backoff + Jitter: 1s, 2s, 4s)                     |   |
|   | -> Success / Final Failure logged to SQLite `webhook_deliveries`                 |   |
|   +----------------------------------------------------------------------------------+   |
+------------------------------------------------------------------------------------------+
```

---

## 2. Event Payload Data Models

```rust
// crates/gitupload-core/src/webhooks.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    GitPush(GitPushPayload),
    GitSyncFailed(GitSyncFailedPayload),
    PrCreated(PrCreatedPayload),
    BatchCompleted(BatchCompletedPayload),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitPushPayload {
    pub repository: String,
    pub branch: String,
    pub commit_hash: String,
    pub commit_message: String,
    pub author_name: String,
    pub author_email: String,
    pub files_changed: usize,
    pub timestamp: DateTime<Utc>,
    pub github_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GitSyncFailedPayload {
    pub repository: String,
    pub branch: String,
    pub error_message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrCreatedPayload {
    pub repository: String,
    pub pr_number: u64,
    pub pr_title: String,
    pub pr_url: String,
    pub author: String,
    pub is_draft: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchCompletedPayload {
    pub total_repos: usize,
    pub successful_repos: usize,
    pub failed_repos: usize,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}
```

---

## 3. Channel Formatters & Transport Implementations

### 3.1 Discord Rich Embed Formatter

```rust
// crates/gitupload-app/src/webhooks/discord.rs
use gitupload_core::webhooks::{GitPushPayload, WebhookEvent};
use serde_json::json;

pub fn format_discord_payload(event: &WebhookEvent) -> serde_json::Value {
    match event {
        WebhookEvent::GitPush(push) => {
            let short_hash = if push.commit_hash.len() >= 7 {
                &push.commit_hash[..7]
            } else {
                &push.commit_hash
            };
            let commit_link = push.github_url.clone().unwrap_or_else(|| "#".to_string());

            json!({
                "username": "GitUpload Desktop",
                "avatar_url": "https://gitupload.org/assets/avatar.png",
                "embeds": [{
                    "title": format!("📦 New Push to {} [{}]", push.repository, push.branch),
                    "description": format!("`{}` - {}", short_hash, push.commit_message),
                    "url": commit_link,
                    "color": 0x2DA44E, // GitHub Primer Green
                    "fields": [
                        { "name": "Author", "value": &push.author_name, "inline": true },
                        { "name": "Files Changed", "value": push.files_changed.to_string(), "inline": true }
                    ],
                    "timestamp": push.timestamp.to_rfc3339()
                }]
            })
        }
        WebhookEvent::GitSyncFailed(fail) => {
            json!({
                "username": "GitUpload Desktop",
                "embeds": [{
                    "title": format!("🚨 Sync Failure in {}", fail.repository),
                    "description": format!("```\n{}\n```", fail.error_message),
                    "color": 0xCF222E, // Primer Red
                    "timestamp": fail.timestamp.to_rfc3339()
                }]
            })
        }
        _ => json!({ "content": "GitUpload Event Triggered" }),
    }
}
```

### 3.2 Slack Block Kit Formatter

```rust
// crates/gitupload-app/src/webhooks/slack.rs
use gitupload_core::webhooks::{PrCreatedPayload, WebhookEvent};
use serde_json::json;

pub fn format_slack_payload(event: &WebhookEvent) -> serde_json::Value {
    match event {
        WebhookEvent::PrCreated(pr) => {
            json!({
                "blocks": [
                    {
                        "type": "header",
                        "text": {
                            "type": "plain_text",
                            "text": format!("🎉 Pull Request Created: #{} {}", pr.pr_number, pr.pr_title),
                            "emoji": true
                        }
                    },
                    {
                        "type": "section",
                        "fields": [
                            { "type": "mrkdwn", "text": format!("*Repository:*\n{}", pr.repository) },
                            { "type": "mrkdwn", "text": format!("*Author:*\n{}", pr.author) }
                        ]
                    },
                    {
                        "type": "actions",
                        "elements": [
                            {
                                "type": "button",
                                "text": { "type": "plain_text", "text": "View on GitHub", "emoji": true },
                                "url": pr.pr_url,
                                "style": "primary"
                            }
                        ]
                    }
                ]
            })
        }
        _ => json!({ "text": "GitUpload notification" }),
    }
}
```

### 3.3 Generic HTTP Dispatcher with HMAC-SHA256 Signing

```rust
// crates/gitupload-app/src/webhooks/http.rs
use ring::hmac;
use secrecy::ExposeSecret;
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
    secret: Option<&secrecy::SecretString>,
) -> Result<u16, WebhookDeliveryError> {
    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("User-Agent", "GitUpload-Dispatcher/2.6")
        .header("X-GitUpload-Event", event_type)
        .header("X-GitUpload-Delivery", uuid::Uuid::new_v4().to_string());

    if let Some(sec) = secret {
        let key = hmac::Key::new(hmac::HMAC_SHA256, sec.expose_secret().as_bytes());
        let signature = hmac::sign(&key, payload_json.as_bytes());
        let hex_sig = format!("sha256={}", hex::encode(signature.as_ref()));
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
```

---

## 4. SQLite Schema & Exponential Backoff

### 4.1 Database DDL
```sql
-- migrations/0008_webhooks.sql
CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    transport TEXT NOT NULL CHECK(transport IN ('discord', 'slack', 'telegram', 'generic')),
    target_url TEXT NOT NULL,
    vault_secret_key TEXT, -- Keyring reference identifier (never raw secret)
    events TEXT NOT NULL,   -- Comma-separated event list e.g. 'git_push,pr_created'
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY,
    endpoint_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    status_code INTEGER,
    attempt_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    error_message TEXT,
    delivered_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(endpoint_id) REFERENCES webhook_endpoints(id) ON DELETE CASCADE
);
```

---

## 5. Work Breakdown & Granular Tasks

### TASK-P5-2-01: Webhook Models & Storage Schema
- **Target Files:**  
  `rust-gitupload/crates/gitupload-core/src/webhooks.rs`  
  `rust-gitupload/crates/gitupload-storage/migrations/0008_webhooks.sql`  
  `rust-gitupload/crates/gitupload-storage/src/webhooks.rs`
- **Actions:**
  - Define serde models for `WebhookEvent`, `GitPushPayload`, `PrCreatedPayload`, and `BatchCompletedPayload`.
  - Add SQLite migrations for `webhook_endpoints` and `webhook_deliveries`.
  - Provide CRUD queries for webhook configuration.

### TASK-P5-2-02: Multi-Platform Formatters
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/webhooks/discord.rs`  
  `rust-gitupload/crates/gitupload-app/src/webhooks/slack.rs`  
  `rust-gitupload/crates/gitupload-app/src/webhooks/telegram.rs`
- **Actions:**
  - Build Discord embed formatter matching Primer branding.
  - Build Slack Block Kit formatter with actionable URL buttons.
  - Implement Telegram MarkdownV2 formatter with strict escaping (`_`, `*`, `[`, `]`, `(`, `)`, `~`, `` ` ``, `>`, `#`, `+`, `-`, `=`, `|`, `{`, `}`, `.`, `!`).

### TASK-P5-2-03: Generic HTTP Dispatcher & HMAC-SHA256 Signing
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/webhooks/http.rs`  
  `rust-gitupload/crates/gitupload-security/src/hmac.rs`
- **Actions:**
  - Implement HMAC-SHA256 signature generator using `ring::hmac`.
  - Fetch webhook signing secrets from OS Keyring via `gitupload-security`.
  - Attach `X-Hub-Signature-256`, `X-GitUpload-Event`, and `X-GitUpload-Delivery` headers.

### TASK-P5-2-04: Asynchronous Dispatcher Queue & Retry Worker
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/webhooks/queue.rs`  
  `rust-gitupload/crates/gitupload-app/src/webhooks/retry.rs`
- **Actions:**
  - Spawn background Tokio worker consuming from `mpsc::channel(256)`.
  - Implement exponential backoff: 3 attempts with delays of 1s, 2s, and 4s plus random jitter (±200ms).
  - Handle HTTP 429 rate limits by honoring the `Retry-After` header.
  - Record audit log entry in `webhook_deliveries` upon completion.

### TASK-P5-2-05: Tauri IPC Commands & Primer Settings UI
- **Target Files:**  
  `rust-gitupload/crates/gitupload-app/src/commands/webhooks.rs`  
  `rust-gitupload/ui/src/components/settings/WebhooksSettings.tsx`
- **Actions:**
  - Expose IPC commands: `webhook_list_endpoints`, `webhook_create_endpoint`, `webhook_delete_endpoint`, `webhook_test_ping`.
  - Provide Primer UI form with transport dropdown, URL field, secret input, and "Send Test Ping" verification button.

---

## 6. Acceptance Criteria (Given-When-Then)

### AC-P5-2-01: Discord Rich Embed Dispatch on Git Push
- **Given:** A configured Discord webhook endpoint listening for `git_push` events.
- **When:** A user executes a push to `origin/main` containing 3 files.
- **Then:** The dispatcher formats a JSON embed with Primer Green color (`0x2DA44E`), commit hash link, and sends an HTTP POST resulting in status code 204.

### AC-P5-2-02: Slack Block Kit Formatting on PR Creation
- **Given:** A configured Slack webhook endpoint listening for `pr_created` events.
- **When:** GitUpload creates Pull Request #42 on GitHub.
- **Then:** A Block Kit message containing the PR title, author, and interactive "View on GitHub" action button is successfully delivered to the target Slack channel.

### AC-P5-2-03: Telegram MarkdownV2 Escaping & Message Delivery
- **Given:** A Telegram bot endpoint with a specified Chat ID.
- **When:** A commit message contains special characters such as `fix(auth): update v1.2 -> v1.3!`.
- **Then:** All reserved characters are escaped prior to payload transmission, preventing Telegram API 400 Bad Request errors.

### AC-P5-2-04: HMAC-SHA256 Signature Verification for Generic Webhooks
- **Given:** A generic HTTP endpoint configured with a pre-shared secret `SuperSecretKey123!`.
- **When:** An event is dispatched to the endpoint.
- **Then:** The HTTP request includes header `X-Hub-Signature-256: sha256=<hex>`, and the receiver can cryptographically verify the signature against the payload body.

### AC-P5-2-05: Exponential Backoff Retry on Transient Network Errors
- **Given:** A webhook target experiencing temporary 503 Service Unavailable errors.
- **When:** The dispatcher attempts delivery.
- **Then:** The dispatcher retries after 1s and 2s before succeeding on the third attempt, updating `attempt_count: 3` and `status_code: 200` in SQLite audit logs.

---

*Document Author:* `@srs_author`  
*Reviewers & Verifiers:* `@verifier`, `@coordinator`  
*Target Release:* GitUpload v2.6.0-extensible
