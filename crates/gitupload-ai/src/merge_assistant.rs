//! Cloud AI 3-way merge conflict assistant producing clean syntax-preserving merged code hunks.

use crate::client::{ChatMessage, CloudAiProvider, CloudAiService};
use gitupload_core::errors::AiResult;
use gitupload_core::models::ConflictHunk;
use secrecy::SecretString;

/// Cloud AI assistant that synthesizes reconciled code blocks for complex semantic merge conflicts.
pub struct MergeAssistant;

impl MergeAssistant {
    /// Streams an AI-proposed conflict resolution for a single [`ConflictHunk`].
    pub async fn auto_resolve_conflict(
        service: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        hunk: &ConflictHunk,
    ) -> AiResult<String> {
        let base_section = hunk
            .content_base
            .as_deref()
            .map(|b| format!("\n--- COMMON ANCESTOR (BASE) ---\n{b}\n"))
            .unwrap_or_default();

        let prompt = format!(
            "Resolve the following 3-way Git merge conflict in `{}` cleanly.\n\
             Return ONLY the exact resolved source code block with zero markdown fences and zero commentary.\n\
             {base_section}\
             --- LOCAL BRANCH (OURS) ---\n{}\n\
             --- INCOMING BRANCH (THEIRS) ---\n{}",
            hunk.file_path, hunk.content_ours, hunk.content_theirs
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an expert senior software engineer resolving 3-way Git merge conflicts. Output only clean valid code with no ``` fences or explanations.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ];

        let candidates = crate::generators::resolve_candidates(provider, api_key, model);
        let raw = service
            .complete_with_failover(&candidates, &messages, |_| {})
            .await?;

        Ok(strip_markdown_fences(&raw))
    }
}

/// Removes optional surrounding markdown triple-backtick fences returned by LLMs.
pub fn strip_markdown_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_lang = rest.split_once('\n').map(|(_, body)| body).unwrap_or(rest);
        if let Some(stripped) = after_lang.strip_suffix("```") {
            return stripped.trim().to_string();
        }
    }
    trimmed.to_string()
}
