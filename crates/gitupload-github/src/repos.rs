//! Repository management, search, file explorer, sanitized README rendering, branch protection, and token diagnostics.

use crate::client::{extract_github_error_message, map_status_error, GitHubClient};
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_core::models::{
    BranchProtectionRule, ContentInfo, FileContentInfo, GitHubProfile, RepositoryInfo, TokenReport,
};
use gitupload_security::sanitizer::sanitize_log_message;
use regex::Regex;
use reqwest::header::ACCEPT;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

static RE_SCRIPT_TAG: OnceLock<Option<Regex>> = OnceLock::new();
static RE_IFRAME_TAG: OnceLock<Option<Regex>> = OnceLock::new();
static RE_EVENT_HANDLER: OnceLock<Option<Regex>> = OnceLock::new();
static RE_JS_URI: OnceLock<Option<Regex>> = OnceLock::new();

/// Sanitizes GitHub-rendered HTML against XSS (SEC-05) by stripping `<script>`, `<iframe>`,
/// inline `on*` handlers, and `javascript:` URI schemes.
pub fn sanitize_readme_html(raw_html: &str) -> String {
    let no_script = RE_SCRIPT_TAG
        .get_or_init(|| Regex::new(r"(?is)<script\b[^>]*>.*?</script>").ok())
        .as_ref()
        .map(|re| re.replace_all(raw_html, "").into_owned())
        .unwrap_or_else(|| raw_html.to_string());

    let no_iframe = RE_IFRAME_TAG
        .get_or_init(|| Regex::new(r"(?is)<(?:iframe|object|embed)\b[^>]*>.*?</(?:iframe|object|embed)>|<(?:iframe|object|embed)\b[^>]*/?>").ok())
        .as_ref()
        .map(|re| re.replace_all(&no_script, "").into_owned())
        .unwrap_or(no_script);

    let no_events = RE_EVENT_HANDLER
        .get_or_init(|| Regex::new(r#"(?i)\s+on[a-z]+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)"#).ok())
        .as_ref()
        .map(|re| re.replace_all(&no_iframe, "").into_owned())
        .unwrap_or(no_iframe);

    RE_JS_URI
        .get_or_init(|| Regex::new(r"(?i)javascript\s*:").ok())
        .as_ref()
        .map(|re| re.replace_all(&no_events, "blocked:").into_owned())
        .unwrap_or(no_events)
}

#[derive(Debug, Serialize)]
struct CreateRepoPayload<'a> {
    name: &'a str,
    private: bool,
}

#[derive(Debug, Serialize)]
struct UpdateFilePayload<'a> {
    message: &'a str,
    content: &'a str,
    sha: &'a str,
}

#[derive(Debug, Deserialize)]
struct SearchReposApiResult {
    total_count: usize,
    items: Vec<RepositoryInfo>,
}

/// Paginated repository search output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRepositoriesResult {
    /// Matching repositories for the requested page.
    pub repos: Vec<RepositoryInfo>,
    /// Total repository count reported by GitHub.
    pub total_count: usize,
}

impl GitHubClient {
    /// Fetches detailed metadata for `{owner}/{repo}`, returning `Ok(None)` on 404.
    pub async fn get_repository(
        &self,
        owner: &str,
        repo: &str,
    ) -> GitHubResult<Option<RepositoryInfo>> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        let resp = self
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }
        let info: RepositoryInfo =
            serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))?;
        Ok(Some(info))
    }

    /// Creates a personal or organization repository, automatically reusing an existing empty repo (`size == 0`) on 422.
    pub async fn create_repository(
        &self,
        owner: &str,
        repo_name: &str,
        is_org: bool,
        private: bool,
    ) -> GitHubResult<RepositoryInfo> {
        let url = if is_org {
            format!("{}/orgs/{}/repos", self.base_url, owner)
        } else {
            format!("{}/user/repos", self.base_url)
        };

        let payload = CreateRepoPayload {
            name: repo_name,
            private,
        };
        let resp = self
            .request(Method::POST, &url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if status == StatusCode::CREATED || status == StatusCode::OK {
            return serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()));
        }

        if status == StatusCode::UNPROCESSABLE_ENTITY {
            let err_msg = extract_github_error_message(&body);
            if err_msg.to_lowercase().contains("already exists") {
                if let Some(existing) = self.get_repository(owner, repo_name).await? {
                    if existing.is_empty() {
                        return Ok(existing);
                    }
                    return Err(GitHubError::Api {
                        status: 422,
                        message: format!(
                            "Repository '{owner}/{repo_name}' already exists and is non-empty"
                        ),
                    });
                }
            }
        }

        Err(map_status_error(status, &body))
    }

    /// Searches or lists repositories for `owner` with pagination and client-side sorting (`"empty"`, `"new"`, `"default"`).
    pub async fn search_repositories(
        &self,
        owner: &str,
        query: &str,
        is_org: bool,
        page: usize,
        page_size: usize,
        sort_by: &str,
    ) -> GitHubResult<SearchRepositoriesResult> {
        let trimmed = query.trim();
        let (mut repos, total_count) = if !trimmed.is_empty() {
            let scope = if is_org { "org" } else { "user" };
            let encoded_q = url::form_urlencoded::byte_serialize(
                format!("{trimmed} {scope}:{owner}").as_bytes(),
            )
            .collect::<String>();
            let url = format!(
                "{}/search/repositories?q={}&per_page={}&page={}&sort=updated",
                self.base_url, encoded_q, page_size, page
            );
            let res: SearchReposApiResult = self.get(&url).await?;
            (res.items, res.total_count)
        } else {
            let url = if is_org {
                format!(
                    "{}/orgs/{}/repos?per_page={}&sort=updated&page={}",
                    self.base_url, owner, page_size, page
                )
            } else {
                format!(
                    "{}/user/repos?per_page={}&sort=updated&affiliation=owner&page={}",
                    self.base_url, page_size, page
                )
            };
            let list: Vec<RepositoryInfo> = self.get(&url).await?;
            let count = list.len();
            (list, count)
        };

        sort_repositories(&mut repos, sort_by);
        Ok(SearchRepositoriesResult { repos, total_count })
    }

    /// Deletes a remote repository (`DELETE /repos/{owner}/{repo}`).
    pub async fn delete_repository(&self, owner: &str, repo: &str) -> GitHubResult<()> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        self.delete_no_content(&url).await
    }

    /// Fetches directory entries at `path` inside `{owner}/{repo}`.
    pub async fn get_repo_contents(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> GitHubResult<Vec<ContentInfo>> {
        let clean = path.trim_matches('/');
        let url = if clean.is_empty() {
            format!("{}/repos/{}/{}/contents", self.base_url, owner, repo)
        } else {
            format!(
                "{}/repos/{}/{}/contents/{}",
                self.base_url, owner, repo, clean
            )
        };
        let resp = self
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }

        if body.first() == Some(&b'{') {
            let single: ContentInfo =
                serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))?;
            Ok(vec![single])
        } else {
            serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))
        }
    }

    /// Fetches and XSS-sanitizes the HTML-rendered `README.md` of `{owner}/{repo}`.
    pub async fn get_repo_readme(&self, owner: &str, repo: &str) -> GitHubResult<Option<String>> {
        let url = format!("{}/repos/{}/{}/readme", self.base_url, owner, repo);
        let resp = self
            .request(Method::GET, &url)
            .header(ACCEPT, "application/vnd.github.html")
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }

        let html = String::from_utf8_lossy(&body);
        Ok(Some(sanitize_readme_html(&html)))
    }

    /// Fetches Base64 content and SHA of a single remote file.
    pub async fn get_repo_file(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> GitHubResult<FileContentInfo> {
        let clean = path.trim_matches('/');
        let url = format!(
            "{}/repos/{}/{}/contents/{}",
            self.base_url, owner, repo, clean
        );
        self.get(&url).await
    }

    /// Updates a remote file atomically using its current blob SHA.
    pub async fn update_repo_file(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        content_base64: &str,
        sha: &str,
        commit_msg: &str,
    ) -> GitHubResult<()> {
        let clean = path.trim_matches('/');
        let url = format!(
            "{}/repos/{}/{}/contents/{}",
            self.base_url, owner, repo, clean
        );
        let payload = UpdateFilePayload {
            message: commit_msg,
            content: content_base64,
            sha,
        };
        let _: serde_json::Value = self.put(&url, &payload).await?;
        Ok(())
    }

    /// Retrieves language byte breakdown for `{owner}/{repo}`.
    pub async fn get_repo_languages(
        &self,
        owner: &str,
        repo: &str,
    ) -> GitHubResult<HashMap<String, u64>> {
        let url = format!("{}/repos/{}/{}/languages", self.base_url, owner, repo);
        self.get(&url).await
    }

    /// Retrieves authenticated user profile (`GET /user`).
    pub async fn get_profile(&self) -> GitHubResult<GitHubProfile> {
        let url = format!("{}/user", self.base_url);
        self.get(&url).await
    }

    /// Inspects token OAuth scopes, rate limits, 2FA status, and repository quotas.
    pub async fn get_token_report(&self) -> GitHubResult<TokenReport> {
        let url = format!("{}/user", self.base_url);
        let resp = self
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }

        let scopes: Vec<String> = headers
            .get("x-oauth-scopes")
            .and_then(|v| v.to_str().ok())
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let rate_limit: u64 = headers
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let rate_remaining: u64 = headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let reset_ts: i64 = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let rate_reset = DateTime::<Utc>::from_timestamp(reset_ts, 0)
            .map(|dt| dt.format("%H:%M:%S UTC").to_string())
            .unwrap_or_default();

        let raw: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))?;

        let has_repo_scope = scopes.iter().any(|s| s == "repo");
        let has_delete_scope = scopes.iter().any(|s| s == "delete_repo");
        let has_org_scope = scopes
            .iter()
            .any(|s| s == "admin:org" || s == "write:org" || s == "read:org");

        Ok(TokenReport {
            scopes,
            rate_limit,
            rate_remaining,
            rate_reset,
            public_repos: raw["public_repos"].as_u64().unwrap_or(0),
            private_repos: raw["total_private_repos"].as_u64().unwrap_or(0),
            disk_usage: raw["disk_usage"].as_u64().unwrap_or(0),
            plan_name: raw["plan"]["name"].as_str().unwrap_or("free").to_string(),
            two_factor_authentication: raw["two_factor_authentication"].as_bool().unwrap_or(false),
            has_repo_scope,
            has_delete_scope,
            has_org_scope,
            username: raw["login"].as_str().unwrap_or("").to_string(),
            avatar_url: raw["avatar_url"].as_str().unwrap_or("").to_string(),
            email: raw["email"].as_str().map(String::from),
        })
    }

    /// Queries branch protection policy for `{owner}/{repo}/branches/{branch}/protection`.
    pub async fn get_branch_protection(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> GitHubResult<BranchProtectionRule> {
        let url = format!(
            "{}/repos/{}/{}/branches/{}/protection",
            self.base_url, owner, repo, branch
        );
        let resp = self
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(BranchProtectionRule {
                branch: branch.to_string(),
                protected: false,
                enforce_admins: false,
                require_pr_reviews: false,
                required_approvals: 0,
                require_linear_history: false,
                allow_force_pushes: true,
            });
        }

        let status = resp.status();
        let body = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;
        if !status.is_success() {
            return Err(map_status_error(status, &body));
        }

        let raw: serde_json::Value =
            serde_json::from_slice(&body).map_err(|e| GitHubError::Parse(e.to_string()))?;

        let reviews = &raw["required_pull_request_reviews"];
        Ok(BranchProtectionRule {
            branch: branch.to_string(),
            protected: true,
            enforce_admins: raw["enforce_admins"]["enabled"].as_bool().unwrap_or(false),
            require_pr_reviews: !reviews.is_null(),
            required_approvals: reviews["required_approving_review_count"]
                .as_u64()
                .unwrap_or(0) as u8,
            require_linear_history: raw["required_linear_history"]["enabled"]
                .as_bool()
                .unwrap_or(false),
            allow_force_pushes: raw["allow_force_pushes"]["enabled"]
                .as_bool()
                .unwrap_or(false),
        })
    }

    /// Syncs a forked repository with its upstream parent branch (`POST /repos/{owner}/{repo}/merge-upstream`).
    pub async fn sync_fork_upstream(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> GitHubResult<String> {
        let url = format!("{}/repos/{}/{}/merge-upstream", self.base_url, owner, repo);
        let payload = serde_json::json!({ "branch": branch });
        let resp: serde_json::Value = self.post(&url, &payload).await?;
        Ok(resp["message"]
            .as_str()
            .unwrap_or("Synced upstream")
            .to_string())
    }
}

/// Sorts repositories by `"empty"`, `"new"`, or preserves default order.
pub fn sort_repositories(repos: &mut [RepositoryInfo], sort_by: &str) {
    match sort_by {
        "empty" => repos.sort_by(|a, b| {
            let a_empty = a.is_empty();
            let b_empty = b.is_empty();
            b_empty
                .cmp(&a_empty)
                .then_with(|| b.pushed_at.cmp(&a.pushed_at))
        }),
        "new" => repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at)),
        _ => {}
    }
}
