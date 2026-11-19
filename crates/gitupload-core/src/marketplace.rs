use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCategory {
    Linter,
    AiPrompt,
    UiTheme,
    Webhook,
    Automation,
}

impl std::str::FromStr for PluginCategory {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "linter" => Ok(Self::Linter),
            "ai_prompt" | "aiprompt" => Ok(Self::AiPrompt),
            "ui_theme" | "uitheme" | "theme" => Ok(Self::UiTheme),
            "webhook" => Ok(Self::Webhook),
            "automation" => Ok(Self::Automation),
            other => Err(format!("Unknown plugin category: {other}")),
        }
    }
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linter => write!(f, "linter"),
            Self::AiPrompt => write!(f, "ai_prompt"),
            Self::UiTheme => write!(f, "ui_theme"),
            Self::Webhook => write!(f, "webhook"),
            Self::Automation => write!(f, "automation"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplacePluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: PluginCategory,
    pub download_url: String,
    pub sha256: String,
    pub ed25519_signature: String,
    pub publisher_pubkey: String,
    pub min_app_version: String,
    pub capabilities: Vec<String>,
    pub verified: bool,
    pub downloads_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketplaceIndex {
    pub version: u32,
    pub updated_at: DateTime<Utc>,
    pub plugins: Vec<MarketplacePluginEntry>,
}
