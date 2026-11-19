//! Application preferences and repository naming configuration models.
//! Note: Sensitive credentials (GitHub PATs, AI API keys) MUST NEVER be fields on this struct.

use serde::{Deserialize, Serialize};

fn default_naming_pattern() -> String {
    "session%02d-ex%02d-homework".to_string()
}

fn default_theme_mode() -> String {
    "dark".to_string()
}

fn default_commit_msg() -> String {
    "init".to_string()
}

fn default_file_commit_pattern() -> String {
    "{filename_no_ext}".to_string()
}

fn default_ai_provider() -> String {
    "xkiro".to_string()
}

fn default_ai_model() -> String {
    "mistralai/codestral-2508".to_string()
}

fn default_language() -> String {
    "vi-VN".to_string()
}

fn default_true() -> bool {
    true
}

/// Naming pattern configuration for repositories and single-file homework commits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamingPattern {
    /// Repository naming template (e.g. `"session%02d-ex%02d-homework"`).
    #[serde(default = "default_naming_pattern")]
    pub pattern: String,
    /// Commit message template for single-file uploads.
    #[serde(default = "default_file_commit_pattern")]
    pub file_commit_pattern: String,
    /// Whether to slugify the generated commit message as the repo name.
    #[serde(default)]
    pub sync_file_repo_name_with_commit: bool,
}

impl Default for NamingPattern {
    fn default() -> Self {
        Self {
            pattern: default_naming_pattern(),
            file_commit_pattern: default_file_commit_pattern(),
            sync_file_repo_name_with_commit: false,
        }
    }
}

/// Non-sensitive application preferences persisted in SQLite `app_settings`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    /// Active GitHub username.
    #[serde(default)]
    pub github_username: String,
    /// Active GitHub organization name (empty for personal account).
    #[serde(default)]
    pub github_org_name: String,
    /// Default naming pattern for `SESSION_EX` mode.
    #[serde(default = "default_naming_pattern")]
    pub naming_pattern: String,
    /// Primer theme identifier (`dark`, `dimmed`, `light`, `high_contrast`).
    #[serde(default = "default_theme_mode")]
    pub theme_mode: String,
    /// Git author `user.name` override.
    #[serde(default)]
    pub git_commit_name: String,
    /// Git author `user.email` override.
    #[serde(default)]
    pub git_commit_email: String,
    /// Create new repositories as private by default.
    #[serde(default)]
    pub repo_private: bool,
    /// Automatically start push after drag-and-drop selection.
    #[serde(default)]
    pub auto_push: bool,
    /// Default commit message when `use_default_commit` is enabled.
    #[serde(default = "default_commit_msg")]
    pub default_commit_msg: String,
    /// Commit message pattern for single-file homework uploads.
    #[serde(default = "default_file_commit_pattern")]
    pub file_commit_msg_pattern: String,
    /// Synchronize single-file repo name with slugified commit message.
    #[serde(default)]
    pub sync_file_repo_name_with_commit: bool,
    /// Selected Cloud AI provider (`gemini`, `openai`, `claude`, `groq`, `openrouter`).
    #[serde(default = "default_ai_provider")]
    pub cloud_ai_provider: String,
    /// Selected Cloud AI model identifier.
    #[serde(default = "default_ai_model")]
    pub cloud_ai_model: String,
    /// UI language (`vi-VN` or `en-US`).
    #[serde(default = "default_language")]
    pub language: String,
    /// Enable safe sync (`--force-with-lease` & collision guard) by default.
    #[serde(default = "default_true")]
    pub safe_sync_enabled: bool,
    /// Automatically stash dirty working tree edits before sync/re-push.
    #[serde(default = "default_true")]
    pub auto_stash_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            github_username: String::new(),
            github_org_name: String::new(),
            naming_pattern: default_naming_pattern(),
            theme_mode: default_theme_mode(),
            git_commit_name: String::new(),
            git_commit_email: String::new(),
            repo_private: false,
            auto_push: false,
            default_commit_msg: default_commit_msg(),
            file_commit_msg_pattern: default_file_commit_pattern(),
            sync_file_repo_name_with_commit: false,
            cloud_ai_provider: default_ai_provider(),
            cloud_ai_model: default_ai_model(),
            language: default_language(),
            safe_sync_enabled: true,
            auto_stash_enabled: true,
        }
    }
}
