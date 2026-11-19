//! High-level Cloud AI orchestrator combining zero-token routing and streaming Cloud LLMs.

use crate::client::{ChatMessage, CloudAiProvider, CloudAiService};
use crate::generators::AiGenerators;
use crate::router::{IntentClassifier, StaticIntent};
use gitupload_core::errors::GitUploadResult;
use secrecy::SecretString;

/// Managed Cloud AI orchestrator stored in Tauri `AppState`.
#[derive(Clone, Default)]
pub struct CloudAIOrchestrator {
    service: CloudAiService,
}

impl CloudAIOrchestrator {
    /// Creates a new `CloudAIOrchestrator`.
    pub fn new() -> Self {
        Self {
            service: CloudAiService::new(),
        }
    }

    /// Returns a reference to the underlying `CloudAiService`.
    pub fn service(&self) -> &CloudAiService {
        &self.service
    }

    /// Generates a Conventional Commit message from `diff` using the configured Cloud AI provider.
    pub async fn generate_commit_msg_with_key(
        &self,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        diff: &str,
    ) -> GitUploadResult<String> {
        let msg =
            AiGenerators::generate_commit_message(&self.service, provider, api_key, model, diff)
                .await?;
        Ok(msg)
    }

    /// Generates a commit message fallback when invoked without an active API key.
    pub async fn generate_commit_msg(&self, diff: &str) -> GitUploadResult<String> {
        let summary_hint = diff
            .lines()
            .find(|l| l.starts_with("+++ b/"))
            .and_then(|l| l.strip_prefix("+++ b/"))
            .unwrap_or("workspace");
        Ok(format!("chore: update {summary_hint}"))
    }

    /// Processes a chat query: checks zero-token intent rules first, falling back to Cloud AI SSE.
    pub async fn chat<F>(
        &self,
        provider: &CloudAiProvider,
        api_key: Option<&SecretString>,
        model: &str,
        history: &[ChatMessage],
        user_input: &str,
        mut on_chunk: F,
    ) -> GitUploadResult<String>
    where
        F: FnMut(&str) + Send,
    {
        let intent = IntentClassifier::classify(user_input);
        if let Some(canned) = IntentClassifier::static_response(&intent) {
            on_chunk(canned);
            return Ok(canned.to_string());
        }

        if let StaticIntent::DirectAction(action) = intent {
            let reply = format!("Đã nhận lệnh: {action:?}");
            on_chunk(&reply);
            return Ok(reply);
        }

        let Some(key) = api_key else {
            return Err(
                gitupload_core::errors::AiError::MissingKey(format!("{provider:?}")).into(),
            );
        };

        let mut msgs = history.to_vec();
        msgs.push(ChatMessage {
            role: "user".into(),
            content: user_input.to_string(),
        });

        let candidates = crate::generators::resolve_candidates(provider, key, model);
        let text = self
            .service
            .complete_with_failover(&candidates, &msgs, on_chunk)
            .await?;
        Ok(text)
    }
}
