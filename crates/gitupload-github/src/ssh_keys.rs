//! GitHub User SSH Public Keys REST API endpoints (`/user/keys`).

use crate::client::{extract_github_error_message, GitHubClient};
use gitupload_core::errors::{GitHubError, GitHubResult};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};

/// GitHub SSH key registration payload (`POST /user/keys`).
#[derive(Debug, Clone, Serialize)]
pub struct RegisterSshKeyRequest<'a> {
    /// Human-readable device title displayed in GitHub SSH Settings.
    pub title: &'a str,
    /// OpenSSH public key line (`ssh-ed25519 AAAA...` or `ssh-rsa AAAA...`).
    pub key: &'a str,
}

/// GitHub SSH key REST response object.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GitHubSshKeyRecord {
    /// GitHub database ID.
    pub id: u64,
    /// Uploaded public key string.
    pub key: String,
    /// Key title.
    pub title: String,
    /// Optional URL.
    #[serde(default)]
    pub url: String,
    /// Creation timestamp ISO string.
    #[serde(default)]
    pub created_at: String,
}

impl GitHubClient {
    /// Uploads an SSH public key to the authenticated user's GitHub account (`POST /user/keys`).
    /// Maps HTTP `422 Unprocessable Entity` (already registered / duplicate key) into a clear descriptive `GitHubError::Api`.
    pub async fn register_ssh_key(
        &self,
        title: &str,
        public_key: &str,
    ) -> GitHubResult<GitHubSshKeyRecord> {
        let url = format!("{}/user/keys", self.base_url);
        let payload = RegisterSshKeyRequest {
            title: title.trim(),
            key: public_key.trim(),
        };

        let resp = self
            .request(Method::POST, &url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| GitHubError::Network(e.to_string()))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(e.to_string()))?;

        if status.is_success() {
            return serde_json::from_slice::<GitHubSshKeyRecord>(&bytes)
                .map_err(|e| GitHubError::Parse(e.to_string()));
        }

        let detail = extract_github_error_message(&bytes);
        if status == StatusCode::UNPROCESSABLE_ENTITY {
            return Err(GitHubError::Api {
                status: 422,
                message: format!("SSH key already registered or invalid on GitHub: {detail}"),
            });
        }

        Err(GitHubError::Api {
            status: status.as_u16(),
            message: detail,
        })
    }

    /// Lists all SSH public keys registered on the authenticated GitHub account (`GET /user/keys`).
    pub async fn list_ssh_keys(&self) -> GitHubResult<Vec<GitHubSshKeyRecord>> {
        let url = format!("{}/user/keys?per_page=100", self.base_url);
        self.get(&url).await
    }

    /// Deletes a registered SSH public key by GitHub ID (`DELETE /user/keys/{key_id}`).
    pub async fn delete_github_ssh_key(&self, github_key_id: u64) -> GitHubResult<()> {
        let url = format!("{}/user/keys/{github_key_id}", self.base_url);
        self.delete_no_content(&url).await
    }
}
