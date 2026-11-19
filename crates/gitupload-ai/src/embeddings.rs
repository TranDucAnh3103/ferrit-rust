//! Cloud AI embeddings API client generating dense normalized `Vec<f32>` vectors for semantic commit search.

use crate::client::{CloudAiProvider, CloudAiService};
use gitupload_core::errors::{AiError, AiResult};
use secrecy::{ExposeSecret, SecretString};

/// Generates semantic vector embeddings via pooled Cloud AI endpoints (`OpenAI`, `Gemini`, or compatible mock/proxy).
pub struct EmbeddingsClient;

impl EmbeddingsClient {
    /// Generates dense `Vec<f32>` embeddings for `text` using the configured Cloud AI provider.
    pub async fn generate_embedding(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        text: &str,
    ) -> AiResult<Vec<f32>> {
        let clean_text = text.replace('\n', " ").trim().to_string();
        if clean_text.is_empty() {
            return Err(AiError::InvalidResponse(
                "Cannot generate embedding for empty text".to_string(),
            ));
        }

        match provider {
            CloudAiProvider::OpenAi
            | CloudAiProvider::OpenRouter
            | CloudAiProvider::OpenAiCompatible(_) => {
                let url = match provider {
                    CloudAiProvider::OpenAiCompatible(custom) => {
                        if custom.contains("/embeddings") {
                            custom.clone()
                        } else {
                            format!("{}/v1/embeddings", custom.trim_end_matches('/'))
                        }
                    }
                    _ => "https://api.openai.com/v1/embeddings".to_string(),
                };
                let payload = serde_json::json!({
                    "model": "text-embedding-3-small",
                    "input": clean_text
                });

                gitupload_core::AirGapGuard::assert_network_allowed(&url)
                    .map_err(|e| AiError::Network(e.to_string()))?;

                let resp = ai
                    .http_client()
                    .post(&url)
                    .header(
                        "Authorization",
                        format!("Bearer {}", api_key.expose_secret()),
                    )
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| AiError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(AiError::ApiError(format!(
                        "Embeddings API error: {}",
                        resp.status()
                    )));
                }

                let val: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

                let vector: Vec<f32> = val["data"][0]["embedding"]
                    .as_array()
                    .ok_or_else(|| {
                        AiError::InvalidResponse("Missing embedding vector in response".into())
                    })?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                Ok(vector)
            }
            CloudAiProvider::GoogleGemini => {
                let url = "https://generativelanguage.googleapis.com/v1beta/models/text-embedding-004:embedContent";
                let payload = serde_json::json!({
                    "model": "models/text-embedding-004",
                    "content": { "parts": [{ "text": clean_text }] }
                });

                gitupload_core::AirGapGuard::assert_network_allowed(url)
                    .map_err(|e| AiError::Network(e.to_string()))?;

                let resp = ai
                    .http_client()
                    .post(url)
                    .header("x-goog-api-key", api_key.expose_secret())
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|e| AiError::Network(e.to_string()))?;

                if !resp.status().is_success() {
                    return Err(AiError::ApiError(format!(
                        "Gemini Embeddings API error: {}",
                        resp.status()
                    )));
                }

                let val: serde_json::Value = resp
                    .json()
                    .await
                    .map_err(|e| AiError::InvalidResponse(e.to_string()))?;

                let vector: Vec<f32> = val["embedding"]["values"]
                    .as_array()
                    .ok_or_else(|| {
                        AiError::InvalidResponse(
                            "Missing embedding values in Gemini response".into(),
                        )
                    })?
                    .iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect();

                Ok(vector)
            }
            _ => Err(AiError::ApiError(
                "Embeddings endpoint unsupported for selected provider; select OpenAI or Gemini"
                    .into(),
            )),
        }
    }
}
