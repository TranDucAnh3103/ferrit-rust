use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookTransport {
    Discord,
    Slack,
    Telegram,
    Generic,
}

impl std::str::FromStr for WebhookTransport {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "discord" => Ok(Self::Discord),
            "slack" => Ok(Self::Slack),
            "telegram" => Ok(Self::Telegram),
            "generic" => Ok(Self::Generic),
            other => Err(format!("Unknown webhook transport: {other}")),
        }
    }
}

impl std::fmt::Display for WebhookTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discord => write!(f, "discord"),
            Self::Slack => write!(f, "slack"),
            Self::Telegram => write!(f, "telegram"),
            Self::Generic => write!(f, "generic"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "payload")]
pub enum WebhookEvent {
    GitPush(GitPushPayload),
    GitSyncFailed(GitSyncFailedPayload),
    PrCreated(PrCreatedPayload),
    BatchCompleted(BatchCompletedPayload),
}

impl WebhookEvent {
    pub fn event_type_name(&self) -> &'static str {
        match self {
            WebhookEvent::GitPush(_) => "git_push",
            WebhookEvent::GitSyncFailed(_) => "git_sync_failed",
            WebhookEvent::PrCreated(_) => "pr_created",
            WebhookEvent::BatchCompleted(_) => "batch_completed",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitSyncFailedPayload {
    pub repository: String,
    pub branch: String,
    pub error_message: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrCreatedPayload {
    pub repository: String,
    pub pr_number: u64,
    pub pr_title: String,
    pub pr_url: String,
    pub author: String,
    pub is_draft: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchCompletedPayload {
    pub total_repos: usize,
    pub successful_repos: usize,
    pub failed_repos: usize,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    pub id: String,
    pub name: String,
    pub transport: WebhookTransport,
    pub target_url: String,
    pub vault_secret_key: Option<String>,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebhookDeliveryRecord {
    pub id: String,
    pub endpoint_id: String,
    pub event_type: String,
    pub status_code: Option<u16>,
    pub attempt_count: u32,
    pub duration_ms: u64,
    pub error_message: Option<String>,
    pub delivered_at: DateTime<Utc>,
}
