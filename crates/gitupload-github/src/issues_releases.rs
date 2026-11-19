//! GitHub Issues, Releases, and Gist management endpoints.

use crate::client::{map_status_error, GitHubClient};
use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_security::sanitizer::sanitize_log_message;
use reqwest::header::CONTENT_TYPE;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Issue summary DTO.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct IssueSummary {
    /// Numeric issue ID.
    pub id: u64,
    /// Repository issue number.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// Markdown body.
    pub body: Option<String>,
    /// Issue state (`"open"`, `"closed"`).
    pub state: String,
    /// Author login.
    pub author: String,
    /// Attached label names.
    pub labels: Vec<String>,
    /// Creation timestamp.
    pub created_at: String,
}

/// Request payload to create an issue.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    /// Issue title.
    pub title: String,
    /// Markdown description.
    pub body: Option<String>,
    /// Optional label names.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Request payload to publish a GitHub Release.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateReleaseRequest {
    /// Target tag name (`v2.2.0`).
    pub tag_name: String,
    /// Display release title.
    pub name: String,
    /// Markdown release notes / AI changelog.
    pub body: String,
    /// Save as draft.
    pub draft: bool,
    /// Mark as pre-release.
    pub prerelease: bool,
}

/// Request payload to create a GitHub Gist.
#[derive(Debug, Serialize, Deserialize)]
pub struct GistCreateRequest {
    /// Gist description.
    pub description: String,
    /// Public visibility flag.
    pub public: bool,
    /// Map of `filename -> content`.
    pub files: HashMap<String, String>,
}

impl GitHubClient {
    /// Lists issues (filtering out pull requests) for `{owner}/{repo}`.
    pub async fn list_issues(&self, owner: &str, repo: &str) -> GitHubResult<Vec<IssueSummary>> {
        let url = format!(
            "{}/repos/{}/{}/issues?state=all&per_page=30",
            self.base_url, owner, repo
        );
        let raw_issues: Vec<serde_json::Value> = self.get(&url).await?;

        let issues = raw_issues
            .into_iter()
            .filter(|i| i["pull_request"].is_null())
            .map(|i| IssueSummary {
                id: i["id"].as_u64().unwrap_or(0),
                number: i["number"].as_u64().unwrap_or(0),
                title: i["title"].as_str().unwrap_or("").to_string(),
                body: i["body"].as_str().map(String::from),
                state: i["state"].as_str().unwrap_or("open").to_string(),
                author: i["user"]["login"].as_str().unwrap_or("").to_string(),
                labels: i["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                created_at: i["created_at"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(issues)
    }

    /// Creates a new GitHub Issue.
    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateIssueRequest,
    ) -> GitHubResult<IssueSummary> {
        let url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);
        let i: serde_json::Value = self.post(&url, req).await?;
        Ok(IssueSummary {
            id: i["id"].as_u64().unwrap_or(0),
            number: i["number"].as_u64().unwrap_or(0),
            title: i["title"].as_str().unwrap_or(&req.title).to_string(),
            body: i["body"].as_str().map(String::from),
            state: i["state"].as_str().unwrap_or("open").to_string(),
            author: i["user"]["login"].as_str().unwrap_or("").to_string(),
            labels: req.labels.clone(),
            created_at: i["created_at"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Closes or reopens an issue (`state`: `"closed"` or `"open"`).
    pub async fn update_issue_state(
        &self,
        owner: &str,
        repo: &str,
        issue_number: u64,
        state: &str,
    ) -> GitHubResult<()> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.base_url, owner, repo, issue_number
        );
        let payload = serde_json::json!({ "state": state });
        let _: serde_json::Value = self.patch(&url, &payload).await?;
        Ok(())
    }

    /// Publishes a GitHub Release and returns its numeric release ID.
    pub async fn create_release(
        &self,
        owner: &str,
        repo: &str,
        req: &CreateReleaseRequest,
    ) -> GitHubResult<u64> {
        let url = format!("{}/repos/{}/{}/releases", self.base_url, owner, repo);
        let resp: serde_json::Value = self.post(&url, req).await?;
        Ok(resp["id"].as_u64().unwrap_or(0))
    }

    /// Uploads a binary asset to an existing GitHub Release upload URL.
    pub async fn upload_release_asset(
        &self,
        upload_url_base: &str,
        release_id: u64,
        asset_name: &str,
        bytes: Vec<u8>,
    ) -> GitHubResult<String> {
        let encoded_name =
            url::form_urlencoded::byte_serialize(asset_name.as_bytes()).collect::<String>();
        let url = format!(
            "{}/releases/{}/assets?name={}",
            upload_url_base.trim_end_matches('/'),
            release_id,
            encoded_name
        );
        let resp = self
            .request(Method::POST, &url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(bytes)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }

        let val: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))?;
        Ok(val["browser_download_url"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    /// Creates a public or secret GitHub Gist and returns its shareable HTML URL.
    pub async fn create_gist(&self, req: &GistCreateRequest) -> GitHubResult<String> {
        let url = format!("{}/gists", self.base_url);
        let files_map: serde_json::Map<String, serde_json::Value> = req
            .files
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::json!({ "content": v })))
            .collect();
        let payload = serde_json::json!({
            "description": req.description,
            "public": req.public,
            "files": files_map,
        });
        let resp: serde_json::Value = self.post(&url, &payload).await?;
        Ok(resp["html_url"].as_str().unwrap_or("").to_string())
    }
}
