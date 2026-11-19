//! Pure Cloud AI multi-provider SSE streaming client (Gemini, OpenAI, Claude, Groq, OpenRouter).

use eventsource_stream::Eventsource;
use futures::StreamExt;
use gitupload_core::errors::{AiError, AiResult};
use gitupload_security::sanitizer::sanitize_log_message;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;

/// Supported Cloud AI providers (zero local LLM bloat).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudAiProvider {
    /// Google Gemini (`generativelanguage.googleapis.com`).
    GoogleGemini,
    /// OpenAI (`api.openai.com`).
    OpenAi,
    /// Anthropic Claude (`api.anthropic.com`).
    AnthropicClaude,
    /// Groq Cloud LPU (`api.groq.com`).
    Groq,
    /// OpenRouter aggregator (`openrouter.ai`).
    OpenRouter,
    /// xKiro Cloud AI (`api.xkiro.com`).
    XKiro,
    /// Custom OpenAI-compatible SSE endpoint (primarily for enterprise proxies or mock test servers).
    OpenAiCompatible(String),
}

/// Free multi-model automatic failover sequence for xKiro (from user screenshots).
pub const XKIRO_FALLBACK_MODELS: &[&str] = &[
    "mistralai/codestral-2508",
    "mistralai/devstral-medium",
    "cohere/north-mini-code",
    "mistralai/mistral-small-2603",
    "cohere/command-a",
    "cohere/command-r7b-12-2024",
    "sensenova/sensenova-6.8-flash-lite",
    "cohere/aya-expanse-32b",
    "mistralai/ministral-14b",
];

impl CloudAiProvider {
    /// Parses a provider name string into `CloudAiProvider`.
    pub fn from_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "gemini" | "google" | "google_gemini" => Self::GoogleGemini,
            "openai" | "gpt" | "gpt4o" => Self::OpenAi,
            "claude" | "anthropic" | "anthropic_claude" => Self::AnthropicClaude,
            "groq" => Self::Groq,
            "xkiro" => Self::XKiro,
            _ => Self::OpenRouter,
        }
    }
}

/// Chat conversation turn passed to Cloud LLM endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// Message role (`"system"`, `"user"`, `"assistant"`).
    pub role: String,
    /// UTF-8 message content.
    pub content: String,
}

/// High-speed streaming Cloud AI HTTP/2 client.
#[derive(Clone)]
pub struct CloudAiService {
    http_client: reqwest::Client,
}

impl Default for CloudAiService {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudAiService {
    /// Constructs a pooled HTTP client with 60-second streaming timeout.
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { http_client }
    }

    /// Exposes the connection-pooled `reqwest::Client` to sibling modules within `gitupload-ai`.
    pub(crate) fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Streams AI completions token-by-token via Server-Sent Events (SSE).
    pub async fn stream_completion<F>(
        &self,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        messages: &[ChatMessage],
        mut on_chunk: F,
    ) -> AiResult<String>
    where
        F: FnMut(&str) + Send,
    {
        let (endpoint, request_body) = build_provider_request(provider, api_key, model, messages);
        gitupload_core::AirGapGuard::assert_network_allowed(&endpoint)
            .map_err(|e| AiError::Network(e.to_string()))?;

        let mut req = self
            .http_client
            .post(&endpoint)
            .header(CONTENT_TYPE, "application/json");

        match provider {
            CloudAiProvider::GoogleGemini => {
                req = req.header("x-goog-api-key", api_key.expose_secret());
            }
            CloudAiProvider::AnthropicClaude => {
                req = req
                    .header("x-api-key", api_key.expose_secret())
                    .header("anthropic-version", "2023-06-01");
            }
            CloudAiProvider::OpenRouter => {
                req = req
                    .header(AUTHORIZATION, format!("Bearer {}", api_key.expose_secret()))
                    .header(
                        "HTTP-Referer",
                        "https://github.com/java-web-service-stephenduc/rust-gitupload",
                    )
                    .header("X-Title", "GitUpload Desktop");
            }
            _ => {
                req = req.header(AUTHORIZATION, format!("Bearer {}", api_key.expose_secret()));
            }
        }

        let resp = req
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AiError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        if !status.is_success() {
            let err_text = sanitize_log_message(&resp.text().await.unwrap_or_default());
            if status.as_u16() == 429 || status.as_u16() == 503 {
                return Err(AiError::RateLimited(format!(
                    "Cloud AI HTTP {}: {}",
                    status, err_text
                )));
            }
            return Err(AiError::ApiError(format!(
                "Cloud AI HTTP {}: {}",
                status, err_text
            )));
        }

        let mut event_stream = resp.bytes_stream().eventsource();
        let mut full_response = String::new();

        while let Some(event_res) = event_stream.next().await {
            match event_res {
                Ok(event) => {
                    if event.data.trim() == "[DONE]" {
                        break;
                    }
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&event.data) {
                        if let Some(delta) = extract_sse_delta(&parsed) {
                            full_response.push_str(delta);
                            on_chunk(delta);
                        }
                    }
                }
                Err(err) => {
                    warn!("SSE stream parse warning: {:?}", err);
                }
            }
        }

        Ok(full_response)
    }

    /// Attempts completion across ordered fallback providers if primary encounters rate limits (429/503) or network errors.
    pub async fn complete_with_failover<F>(
        &self,
        candidates: &[(CloudAiProvider, SecretString, String)],
        messages: &[ChatMessage],
        mut on_chunk: F,
    ) -> AiResult<String>
    where
        F: FnMut(&str) + Send,
    {
        let mut last_err = AiError::MissingKey("No Cloud AI providers configured".into());
        for (provider, key, model) in candidates {
            match self
                .stream_completion(provider, key, model, messages, |chunk| on_chunk(chunk))
                .await
            {
                Ok(text) => return Ok(text),
                Err(err @ AiError::RateLimited(_))
                | Err(err @ AiError::Network(_))
                | Err(err @ AiError::ApiError(_)) => {
                    warn!("Cloud AI failover triggered for model '{}' after error: {}", model, err);
                    last_err = err;
                }
                Err(err) => return Err(err),
            }
        }
        Err(last_err)
    }
}

fn build_provider_request(
    provider: &CloudAiProvider,
    _api_key: &SecretString,
    model: &str,
    messages: &[ChatMessage],
) -> (String, serde_json::Value) {
    match provider {
        CloudAiProvider::XKiro => (
            "https://api.xkiro.com/v1/chat/completions".to_string(),
            serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true
            }),
        ),
        CloudAiProvider::GoogleGemini => {
            let url = format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse",
                model
            );
            let contents: Vec<serde_json::Value> = messages
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "role": if m.role == "assistant" { "model" } else { "user" },
                        "parts": [{ "text": m.content }]
                    })
                })
                .collect();
            (url, serde_json::json!({ "contents": contents }))
        }
        CloudAiProvider::Groq => (
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true
            }),
        ),
        CloudAiProvider::OpenAi => (
            "https://api.openai.com/v1/chat/completions".to_string(),
            serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true
            }),
        ),
        CloudAiProvider::OpenRouter => (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true
            }),
        ),
        CloudAiProvider::AnthropicClaude => {
            let system_prompt: String = messages
                .iter()
                .filter(|m| m.role == "system")
                .map(|m| m.content.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");
            let non_system: Vec<&ChatMessage> =
                messages.iter().filter(|m| m.role != "system").collect();
            (
                "https://api.anthropic.com/v1/messages".to_string(),
                serde_json::json!({
                    "model": model,
                    "max_tokens": 4096,
                    "system": system_prompt,
                    "messages": non_system,
                    "stream": true
                }),
            )
        }
        CloudAiProvider::OpenAiCompatible(endpoint) => (
            endpoint.clone(),
            serde_json::json!({
                "model": model,
                "messages": messages,
                "stream": true
            }),
        ),
    }
}

fn extract_sse_delta(parsed: &serde_json::Value) -> Option<&str> {
    // OpenAI / Groq / OpenRouter format
    if let Some(delta) = parsed["choices"][0]["delta"]["content"].as_str() {
        return Some(delta);
    }
    // Non-stream fallback JSON object
    if let Some(msg) = parsed["choices"][0]["message"]["content"].as_str() {
        return Some(msg);
    }
    // Google Gemini SSE format
    if let Some(text) = parsed["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        return Some(text);
    }
    // Anthropic Claude SSE format (`content_block_delta`)
    if let Some(text) = parsed["delta"]["text"].as_str() {
        return Some(text);
    }
    None
}
