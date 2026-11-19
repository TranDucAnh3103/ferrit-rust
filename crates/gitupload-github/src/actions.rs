//! GitHub Actions workflow run monitoring, rerun triggers, cancellation, and log fetching.

use crate::client::{map_status_error, GitHubClient};
use gitupload_core::errors::{GitHubError, GitHubResult};
use gitupload_security::sanitizer::sanitize_log_message;
use reqwest::Method;
use serde::{Deserialize, Serialize};

/// GitHub Actions workflow run metadata.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct WorkflowRun {
    /// Run numeric ID.
    pub id: u64,
    /// Workflow display name.
    pub name: String,
    /// Branch name.
    pub head_branch: String,
    /// Trigger commit SHA.
    pub head_sha: String,
    /// Trigger event (`push`, `pull_request`, `workflow_dispatch`).
    pub event: String,
    /// Status (`queued`, `in_progress`, `completed`).
    pub status: String,
    /// Conclusion (`success`, `failure`, `cancelled`).
    pub conclusion: Option<String>,
    /// GitHub web URL.
    pub html_url: String,
    /// Creation timestamp.
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    workflow_runs: Vec<WorkflowRun>,
}

impl GitHubClient {
    /// Lists the 20 most recent GitHub Actions workflow runs for `{owner}/{repo}`.
    pub async fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
    ) -> GitHubResult<Vec<WorkflowRun>> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs?per_page=20",
            self.base_url, owner, repo
        );
        let resp: WorkflowRunsResponse = self.get(&url).await?;
        Ok(resp.workflow_runs)
    }

    /// Triggers a rerun of only failed jobs in workflow run `run_id`.
    pub async fn rerun_failed_jobs(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> GitHubResult<()> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}/rerun-failed-jobs",
            self.base_url, owner, repo, run_id
        );
        self.post_empty(&url).await
    }

    /// Triggers a full rerun of all jobs in workflow run `run_id`.
    pub async fn rerun_all_jobs(&self, owner: &str, repo: &str, run_id: u64) -> GitHubResult<()> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}/rerun",
            self.base_url, owner, repo, run_id
        );
        self.post_empty(&url).await
    }

    /// Cancels an active or queued workflow run.
    pub async fn cancel_workflow_run(
        &self,
        owner: &str,
        repo: &str,
        run_id: u64,
    ) -> GitHubResult<()> {
        let url = format!(
            "{}/repos/{}/{}/actions/runs/{}/cancel",
            self.base_url, owner, repo, run_id
        );
        self.post_empty(&url).await
    }

    /// Fetches job log output for a specific Actions job ID (sanitized for ANSI terminal rendering).
    pub async fn get_job_logs(&self, owner: &str, repo: &str, job_id: u64) -> GitHubResult<String> {
        let url = format!(
            "{}/repos/{}/{}/actions/jobs/{}/logs",
            self.base_url, owner, repo, job_id
        );
        let resp = self
            .request(Method::GET, &url)
            .send()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        let status = resp.status();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| GitHubError::Network(sanitize_log_message(&e.to_string())))?;

        if !status.is_success() {
            return Err(map_status_error(status, &bytes));
        }

        Ok(sanitize_log_message(&String::from_utf8_lossy(&bytes)))
    }
}
