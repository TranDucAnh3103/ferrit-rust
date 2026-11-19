//! Shared HTTP/2 connection-pooled GitHub API client with SEC-02 error sanitization.

use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_security::sanitizer::sanitize_log_message;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

const DEFAULT_GITHUB_API: &str = "https://api.github.com";

/// Connection-pooled GitHub REST/GraphQL client holding tokens exclusively as `SecretString`.
#[derive(Clone)]
pub struct GitHubClient {
    client: Client,
    pub(crate) base_url: String,
    token: Option<SecretString>,
}

impl Default for GitHubClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl GitHubClient {
    /// Creates a `GitHubClient` targeting `https://api.github.com` from an optional raw token string.
    pub fn new(token: Option<String>) -> Self {
        Self::from_secret(token.map(SecretString::from))
    }

    /// Creates a `GitHubClient` wrapping a `SecretString` token.
    pub fn from_secret(token: Option<SecretString>) -> Self {
        Self::with_base_url(DEFAULT_GITHUB_API, token)
    }

    /// Creates a `GitHubClient` pointing to a custom base URL (e.g. GitHub Enterprise or `wiremock`).
    pub fn with_base_url(base_url: impl Into<String>, token: Option<SecretString>) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("GitUpload-Desktop/2.2"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token,
        }
    }

    /// Returns a reference to the underlying `reqwest::Client`.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the active API base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns a clone of the client configured with a new `SecretString` token.
    pub fn with_token(&self, token: Option<SecretString>) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            token,
        }
    }

    pub(crate) fn request(&self, method: Method, url: &str) -> RequestBuilder {
        let mut req = self.client.request(method, url);
        if let Some(ref secret) = self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", secret.expose_secret()));
        }
        req
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, url: &str) -> GitHubResult<T> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::GET, url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        decode_json_response(resp).await
    }

    pub(crate) async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> GitHubResult<T> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::POST, url)
            .json(body)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        decode_json_response(resp).await
    }

    pub(crate) async fn put<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> GitHubResult<T> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::PUT, url)
            .json(body)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        decode_json_response(resp).await
    }

    pub(crate) async fn patch<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        body: &B,
    ) -> GitHubResult<T> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::PATCH, url)
            .json(body)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        decode_json_response(resp).await
    }

    pub(crate) async fn post_empty(&self, url: &str) -> GitHubResult<()> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::POST, url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        ensure_success_status(resp).await
    }

    pub(crate) async fn delete_no_content(&self, url: &str) -> GitHubResult<()> {
        gitupload_core::AirGapGuard::assert_network_allowed(url)
            .map_err(|e| GitHubError::Network(e.to_string()))?;
        let resp = self
            .request(Method::DELETE, url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        ensure_success_status(resp).await
    }
}

pub(crate) fn extract_github_error_message(body: &[u8]) -> String {
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(msg) = val["message"].as_str() {
            if !msg.is_empty() {
                return sanitize_log_message(msg);
            }
        }
    }
    if body.is_empty() {
        return "No details returned by GitHub API".into();
    }
    sanitize_log_message(&String::from_utf8_lossy(body))
}

async fn ensure_success_status(resp: reqwest::Response) -> GitHubResult<()> {
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.bytes().await.unwrap_or_default();
    Err(map_status_error(status, &body))
}

async fn decode_json_response<T: DeserializeOwned>(resp: reqwest::Response) -> GitHubResult<T> {
    let status = resp.status();
    let body = resp
        .bytes()
        .await
        .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

    if !status.is_success() {
        return Err(map_status_error(status, &body));
    }

    serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(format!("JSON decode error: {e}")))
}

pub(crate) fn map_status_error(status: StatusCode, body: &[u8]) -> GitHubError {
    let msg = extract_github_error_message(body);
    match status {
        StatusCode::UNAUTHORIZED => GitHubError::Unauthorized(msg),
        StatusCode::FORBIDDEN => GitHubError::Forbidden(msg),
        StatusCode::NOT_FOUND => GitHubError::NotFound(msg),
        other => GitHubError::Api {
            status: other.as_u16(),
            message: msg,
        },
    }
}
