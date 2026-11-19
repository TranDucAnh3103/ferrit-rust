//! Zero-token static intent classifier and `[AGENT_ACTION:...]` parser matching legacy Stephen AI rules.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Zero-token classified user intent evaluated before querying any Cloud AI provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticIntent {
    /// Greeting message (`xin chào`, `hello`, etc.).
    Greeting,
    /// Query about tool author / repository (`ai phát triển`, `tác giả`, etc.).
    AuthorInfo,
    /// Query about voice assistant capability.
    VoiceHelp,
    /// Off-topic chatter unrelated to Git/GitHub/GitUpload.
    OffTopic,
    /// Tab navigation command (`upload`, `graph`, `pulls`, `actions`, `settings`).
    NavigateTab(String),
    /// Direct CLI-like user command (`clone <url>` or `push <folder>`).
    DirectAction(AgentAction),
    /// Requires Cloud LLM reasoning.
    Unknown,
}

/// Structured agent action extracted from user commands or `[AGENT_ACTION:...]` tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentAction {
    /// Switch to Clone tab and clone `url` into optional `dest`.
    Clone { url: String, dest: Option<String> },
    /// Switch to Push tab with optional `folder`.
    Push { folder: Option<String> },
    /// Switch UI tab.
    Navigate { tab: String },
}

static RE_GREETING: OnceLock<Option<Regex>> = OnceLock::new();
static RE_AUTHOR: OnceLock<Option<Regex>> = OnceLock::new();
static RE_VOICE: OnceLock<Option<Regex>> = OnceLock::new();
static RE_ACTION_CLONE: OnceLock<Option<Regex>> = OnceLock::new();
static RE_ACTION_PUSH: OnceLock<Option<Regex>> = OnceLock::new();
static RE_ACTION_NAV: OnceLock<Option<Regex>> = OnceLock::new();
static RE_CMD_CLONE: OnceLock<Option<Regex>> = OnceLock::new();

const OFF_TOPIC_KEYWORDS: &[&str] = &[
    "messi",
    "ronaldo",
    "bóng đá",
    "thời tiết",
    "cầu thủ",
    "chính trị",
    "giá vàng",
    "đô la",
];

const DEV_KEYWORDS: &[&str] = &[
    "git",
    "github",
    "code",
    "push",
    "clone",
    "repo",
    "commit",
    "pattern",
    "gitupload",
    "stephen",
    "tác giả",
    "lỗi",
    "đẩy bài",
];

/// Local zero-token intent router.
pub struct IntentClassifier;

impl IntentClassifier {
    /// Classifies user input locally in sub-microsecond time without incurring LLM token cost.
    pub fn classify(input: &str) -> StaticIntent {
        let trimmed = input.trim();
        let lower = trimmed.to_lowercase();

        if let Some(re) = RE_CMD_CLONE
            .get_or_init(|| Regex::new(r"(?i)^clone\s+(https?://[^\s]+)$").ok())
            .as_ref()
        {
            if let Some(caps) = re.captures(trimmed) {
                if let Some(url) = caps.get(1) {
                    return StaticIntent::DirectAction(AgentAction::Clone {
                        url: url.as_str().to_string(),
                        dest: None,
                    });
                }
            }
        }

        if let Some(_folder) = lower.strip_prefix("push ") {
            let f = trimmed[5..].trim();
            if !f.is_empty() {
                return StaticIntent::DirectAction(AgentAction::Push {
                    folder: Some(f.to_string()),
                });
            }
        } else if lower == "push" {
            return StaticIntent::DirectAction(AgentAction::Push { folder: None });
        }

        if RE_GREETING
            .get_or_init(|| Regex::new(r"^(xin chào|chào|hello|hi|hey|alo)\b").ok())
            .as_ref()
            .is_some_and(|re| re.is_match(&lower))
        {
            return StaticIntent::Greeting;
        }

        if RE_AUTHOR
            .get_or_init(|| {
                Regex::new(r"(ai phát triển|tác giả|ai viết|who made|who developed)").ok()
            })
            .as_ref()
            .is_some_and(|re| re.is_match(&lower))
        {
            return StaticIntent::AuthorInfo;
        }

        if RE_VOICE
            .get_or_init(|| {
                Regex::new(r"(giọng nói|voice|micro|mic).*(hỗ trợ|dùng|tính năng)").ok()
            })
            .as_ref()
            .is_some_and(|re| re.is_match(&lower))
        {
            return StaticIntent::VoiceHelp;
        }

        let is_dev = DEV_KEYWORDS.iter().any(|k| lower.contains(k));
        if !is_dev && OFF_TOPIC_KEYWORDS.iter().any(|k| lower.contains(k)) {
            return StaticIntent::OffTopic;
        }

        StaticIntent::Unknown
    }

    /// Returns canned zero-token response for deterministic `StaticIntent` variants.
    pub fn static_response(intent: &StaticIntent) -> Option<&'static str> {
        match intent {
            StaticIntent::Greeting => Some(
                "Chào bạn! Stephen ở đây để hỗ trợ bạn về GitUpload Desktop, quy trình Git/GitHub, Pull Requests và CI/CD Actions.",
            ),
            StaticIntent::AuthorInfo => Some(
                "GitUpload Desktop được phát triển bởi tác giả Stephen (Trần Đức Anh) — Email: tranducanh31032006@gmail.com | GitHub: https://github.com/java-web-service-stephenduc/rust-gitupload",
            ),
            StaticIntent::VoiceHelp => Some(
                "GitUpload hỗ trợ điều khiển lệnh đẩy bài và tìm kiếm nhanh bằng giọng nói hoặc lệnh chat trực tiếp (`push`, `clone <url>`).",
            ),
            StaticIntent::OffTopic => Some(
                "Stephen chỉ hỗ trợ các câu hỏi liên quan đến GitUpload và quy trình quản lý mã nguồn Git/GitHub.",
            ),
            _ => None,
        }
    }

    /// Extracts `[AGENT_ACTION:...]` directive tokens embedded inside an AI response.
    pub fn parse_agent_action(response_text: &str) -> Option<AgentAction> {
        if let Some(re) = RE_ACTION_CLONE
            .get_or_init(|| {
                Regex::new(r"\[AGENT_ACTION:clone:url=([^:\]]+)(?::dest=([^\]]+))?\]").ok()
            })
            .as_ref()
        {
            if let Some(caps) = re.captures(response_text) {
                return Some(AgentAction::Clone {
                    url: caps.get(1)?.as_str().trim().to_string(),
                    dest: caps.get(2).map(|m| m.as_str().trim().to_string()),
                });
            }
        }

        if let Some(re) = RE_ACTION_PUSH
            .get_or_init(|| Regex::new(r"\[AGENT_ACTION:push:folder=([^\]]+)\]").ok())
            .as_ref()
        {
            if let Some(caps) = re.captures(response_text) {
                return Some(AgentAction::Push {
                    folder: Some(caps.get(1)?.as_str().trim().to_string()),
                });
            }
        }

        if response_text.contains("[AGENT_ACTION:push]") {
            return Some(AgentAction::Push { folder: None });
        }

        if let Some(re) = RE_ACTION_NAV
            .get_or_init(|| Regex::new(r"\[AGENT_ACTION:navigate:tab=([^\]]+)\]").ok())
            .as_ref()
        {
            if let Some(caps) = re.captures(response_text) {
                return Some(AgentAction::Navigate {
                    tab: caps.get(1)?.as_str().trim().to_string(),
                });
            }
        }

        None
    }
}
