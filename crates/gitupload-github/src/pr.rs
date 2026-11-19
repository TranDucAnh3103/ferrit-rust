//! Pull Request (PR) lifecycle API: list, create (standard/draft), review, and merge (`merge`/`squash`/`rebase`).

use crate::client::GitHubClient;
use gitupload_core::errors::GitHubResult;
use serde::{Deserialize, Serialize};

/// Pull Request detail DTO matching Phase 4 specification.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PullRequestInfo {
    /// Numeric PR ID.
    pub id: u64,
    /// Repository PR number.
    pub number: u64,
    /// PR title.
    pub title: String,
    /// Markdown body.
    pub body: Option<String>,
    /// PR state (`"open"`, `"closed"`).
    pub state: String,
    /// Draft PR flag.
    pub draft: bool,
    /// Web URL.
    pub html_url: String,
    /// Author login.
    pub user_login: String,
    /// Head branch name.
    pub head_ref: String,
    /// Base branch name.
    pub base_ref: String,
    /// Mergeable status.
    pub mergeable: Option<bool>,
    /// Comment count.
    pub comments_count: u32,
    /// Creation timestamp string.
    pub created_at: String,
}

/// Request payload for `POST /repos/{owner}/{repo}/pulls`.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePullRequest {
    /// PR title.
    pub title: String,
    /// Head branch.
    pub head: String,
    /// Base branch.
    pub base: String,
    /// Optional PR body.
    pub body: Option<String>,
    /// Draft mode flag.
    pub draft: bool,
}

/// Request payload for `PUT /repos/{owner}/{repo}/pulls/{pull_number}/merge`.
#[derive(Debug, Serialize, Deserialize)]
pub struct MergePullRequest {
    /// Optional merge commit title.
    pub commit_title: Option<String>,
    /// Optional merge commit body.
    pub commit_message: Option<String>,
    /// Merge strategy (`"merge"`, `"squash"`, or `"rebase"`).
    pub merge_method: String,
}

/// Pull Request review decision event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewEvent {
    /// Approve the Pull Request.
    Approve,
    /// Request changes before merging.
    RequestChanges,
    /// Leave a general review comment.
    Comment,
}

#[derive(Debug, Serialize)]
struct SubmitReviewBody<'a> {
    body: &'a str,
    event: ReviewEvent,
}

impl GitHubClient {
    /// Lists Pull Requests for `{owner}/{repo}` filtered by `state` (`"open"`, `"closed"`, `"all"`).
    pub async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: &str,
    ) -> GitHubResult<Vec<PullRequestInfo>> {
        let url = format!(
            "{}/repos/{}/{}/pulls?state={}&per_page=30",
            self.base_url, owner, repo, state
        );
        let raw_prs: Vec<serde_json::Value> = self.get(&url).await?;

        let prs = raw_prs
            .into_iter()
            .map(|p| PullRequestInfo {
                id: p["id"].as_u64().unwrap_or(0),
                number: p["number"].as_u64().unwrap_or(0),
                title: p["title"].as_str().unwrap_or("").to_string(),
                body: p["body"].as_str().map(String::from),
                state: p["state"].as_str().unwrap_or("open").to_string(),
                draft: p["draft"].as_bool().unwrap_or(false),
                html_url: p["html_url"].as_str().unwrap_or("").to_string(),
                user_login: p["user"]["login"].as_str().unwrap_or("").to_string(),
                head_ref: p["head"]["ref"].as_str().unwrap_or("").to_string(),
                base_ref: p["base"]["ref"].as_str().unwrap_or("").to_string(),
                mergeable: p["mergeable"].as_bool(),
                comments_count: p["comments"].as_u64().unwrap_or(0) as u32,
                created_at: p["created_at"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(prs)
    }

    /// Creates a new standard or Draft Pull Request.
    pub async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        req: &CreatePullRequest,
    ) -> GitHubResult<PullRequestInfo> {
        let url = format!("{}/repos/{}/{}/pulls", self.base_url, owner, repo);
        let val: serde_json::Value = self.post(&url, req).await?;

        Ok(PullRequestInfo {
            id: val["id"].as_u64().unwrap_or(0),
            number: val["number"].as_u64().unwrap_or(0),
            title: val["title"].as_str().unwrap_or(&req.title).to_string(),
            body: val["body"]
                .as_str()
                .map(String::from)
                .or_else(|| req.body.clone()),
            state: val["state"].as_str().unwrap_or("open").to_string(),
            draft: val["draft"].as_bool().unwrap_or(req.draft),
            html_url: val["html_url"].as_str().unwrap_or("").to_string(),
            user_login: val["user"]["login"].as_str().unwrap_or("").to_string(),
            head_ref: val["head"]["ref"].as_str().unwrap_or(&req.head).to_string(),
            base_ref: val["base"]["ref"].as_str().unwrap_or(&req.base).to_string(),
            mergeable: val["mergeable"].as_bool(),
            comments_count: 0,
            created_at: val["created_at"].as_str().unwrap_or("").to_string(),
        })
    }

    /// Submits an official PR review (`APPROVE`, `REQUEST_CHANGES`, `COMMENT`).
    pub async fn submit_pr_review(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        body: &str,
        event: ReviewEvent,
    ) -> GitHubResult<u64> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/reviews",
            self.base_url, owner, repo, pr_number
        );
        let payload = SubmitReviewBody { body, event };
        let val: serde_json::Value = self.post(&url, &payload).await?;
        Ok(val["id"].as_u64().unwrap_or(0))
    }

    /// Merges an open Pull Request using `"merge"`, `"squash"`, or `"rebase"`.
    pub async fn merge_pull_request(
        &self,
        owner: &str,
        repo: &str,
        pr_number: u64,
        req: &MergePullRequest,
    ) -> GitHubResult<bool> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/merge",
            self.base_url, owner, repo, pr_number
        );
        let val: serde_json::Value = self.put(&url, req).await?;
        Ok(val["merged"].as_bool().unwrap_or(false))
    }
}
