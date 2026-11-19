//! Cloud AI prompt pipelines: Conventional Commit generator, PR drafter, error diagnostics, and semantic changelog.

use crate::client::{ChatMessage, CloudAiProvider, CloudAiService};
use gitupload_core::errors::AiResult;
use secrecy::SecretString;

/// Stateless Cloud AI task generators for Git and GitHub workflows.
pub struct AiGenerators;

impl AiGenerators {
    /// Generates a Conventional Commits 1.0.0 message from a unified working diff.
    pub async fn generate_commit_message(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        diff_text: &str,
    ) -> AiResult<String> {
        let system_prompt =
            "You are an expert Git commit author. Given a code diff, write a concise, \
            imperative Conventional Commit message complying with Conventional Commits 1.0.0 \
            (<type>(<scope>): <description>, header <= 72 chars). Return ONLY the commit message \
            without commentary or markdown code blocks."
                .to_string();

        let truncated_diff = truncate_utf8(diff_text, 16_384);
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".into(),
                content: format!("Generate a commit message for this diff:\n\n{truncated_diff}"),
            },
        ];

        let candidates = resolve_candidates(provider, api_key, model);
        let raw = ai
            .complete_with_failover(&candidates, &messages, |_| {})
            .await?;
        Ok(clean_markdown_fences(&raw))
    }

    /// Generates structured semantic release notes between two Git tags.
    pub async fn generate_release_changelog(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        tag_name: &str,
        commit_summaries: &[String],
    ) -> AiResult<String> {
        let system_prompt = "You are a software release manager. Categorize the provided commit \
            messages into clean Markdown release notes with sections: '### 🚀 Features', \
            '### 🐛 Bug Fixes', and '### ⚠️ Breaking Changes'."
            .to_string();

        let commit_list = commit_summaries.join("\n- ");
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "Draft release notes for version {tag_name} based on these commits:\n- {commit_list}"
                ),
            },
        ];

        let candidates = resolve_candidates(provider, api_key, model);
        ai.complete_with_failover(&candidates, &messages, |_| {})
            .await
    }

    /// Drafts a concise Markdown Pull Request description from branch diff/commits.
    pub async fn generate_pr_description(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        head_branch: &str,
        base_branch: &str,
        diff_or_commits: &str,
    ) -> AiResult<String> {
        let truncated = truncate_utf8(diff_or_commits, 16_384);
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "Write a clear GitHub Pull Request description in Markdown with '## Summary', '## Changes', and '## Test Plan' sections.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "Draft a PR merging `{head_branch}` into `{base_branch}` based on:\n\n{truncated}"
                ),
            },
        ];

        let candidates = resolve_candidates(provider, api_key, model);
        ai.complete_with_failover(&candidates, &messages, |_| {})
            .await
    }

    /// Diagnoses a Git/GitHub error and suggests actionable remediation steps.
    pub async fn diagnose_git_error(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        error_text: &str,
        console_logs: &str,
    ) -> AiResult<String> {
        let messages = vec![
            ChatMessage {
                role: "system".into(),
                content: "You are Stephen, an expert GitUpload assistant. Explain the Git error concisely in Vietnamese or English and give exact remediation steps.".into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!(
                    "Error: {error_text}\n\nRecent logs:\n{}",
                    truncate_utf8(console_logs, 4096)
                ),
            },
        ];

        let candidates = resolve_candidates(provider, api_key, model);
        ai.complete_with_failover(&candidates, &messages, |_| {})
            .await
    }
}

pub(crate) fn resolve_candidates(
    provider: &CloudAiProvider,
    api_key: &SecretString,
    model: &str,
) -> Vec<(CloudAiProvider, SecretString, String)> {
    if provider == &CloudAiProvider::XKiro {
        let mut models = vec![model.to_string()];
        for &m in crate::client::XKIRO_FALLBACK_MODELS {
            if m != model {
                models.push(m.to_string());
            }
        }
        models
            .into_iter()
            .map(|m| (provider.clone(), api_key.clone(), m))
            .collect()
    } else {
        vec![(provider.clone(), api_key.clone(), model.to_string())]
    }
}

fn truncate_utf8(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn clean_markdown_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("```")
        .and_then(|rest| rest.split_once('\n').map(|(_, body)| body))
        .unwrap_or(trimmed);
    stripped.trim_end_matches("```").trim().to_string()
}
