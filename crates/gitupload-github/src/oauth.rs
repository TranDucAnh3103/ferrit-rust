//! GitHub OAuth 2.0 Device Code Flow (`FR-GH-002`) eliminating manual PAT copying.

use crate::client::GitHubClient;
use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_security::sanitizer::sanitize_log_message;
use reqwest::header::ACCEPT;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// Response returned by `POST /login/device/code`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// Device verification code used during polling.
    pub device_code: String,
    /// Short 8-character code shown to the user.
    pub user_code: String,
    /// Verification browser URL (`https://github.com/login/device`).
    pub verification_uri: String,
    /// Lifetime in seconds.
    pub expires_in: u64,
    /// Minimum polling interval in seconds.
    pub interval: u64,
}

/// Polling outcome for OAuth Device Code authorization.
#[derive(Debug)]
pub enum OAuthPollResult {
    /// User has not yet authorized; keep polling after `interval` seconds.
    Pending,
    /// User authorized; token wrapped securely in `SecretString`.
    Authorized(SecretString),
}

impl GitHubClient {
    /// Requests a GitHub OAuth Device Code for the specified `client_id` and `scope`.
    pub async fn request_device_code(
        &self,
        oauth_base_url: &str,
        client_id: &str,
        scope: &str,
    ) -> GitHubResult<DeviceCodeResponse> {
        let url = format!("{}/login/device/code", oauth_base_url.trim_end_matches('/'));
        let resp = self
            .client()
            .post(&url)
            .header(ACCEPT, "application/json")
            .form(&[("client_id", client_id), ("scope", scope)])
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(GitHubError::Api {
                status: status.as_u16(),
                message: sanitize_log_message(&String::from_utf8_lossy(&body)),
            });
        }
        serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))
    }

    /// Polls `POST /login/oauth/access_token` once for device authorization completion.
    pub async fn poll_device_access_token(
        &self,
        oauth_base_url: &str,
        client_id: &str,
        device_code: &str,
    ) -> GitHubResult<OAuthPollResult> {
        let url = format!(
            "{}/login/oauth/access_token",
            oauth_base_url.trim_end_matches('/')
        );
        let resp = self
            .client()
            .post(&url)
            .header(ACCEPT, "application/json")
            .form(&[
                ("client_id", client_id),
                ("device_code", device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))?;

        if let Some(tok) = val["access_token"].as_str() {
            if !tok.is_empty() {
                return Ok(OAuthPollResult::Authorized(SecretString::from(
                    tok.to_string(),
                )));
            }
        }

        let err_code = val["error"].as_str().unwrap_or("");
        if err_code == "authorization_pending" || err_code == "slow_down" {
            return Ok(OAuthPollResult::Pending);
        }

        Err(GitHubError::Api {
            status: 400,
            message: format!("OAuth device poll failed: {err_code}"),
        })
    }
}
