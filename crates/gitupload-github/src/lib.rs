#![forbid(unsafe_code)]
//! GitHub REST and GraphQL API client for repositories, organizations, PRs, Actions, Issues, Releases, and SSH keys.

pub mod actions;
pub mod client;
pub mod issues_releases;
pub mod oauth;
pub mod orgs;
pub mod pr;
pub mod proxy;
pub mod repos;
pub mod squash_reconciler;
pub mod ssh_keys;

pub use actions::WorkflowRun;
pub use client::GitHubClient;
pub use issues_releases::{
    CreateIssueRequest, CreateReleaseRequest, GistCreateRequest, IssueSummary,
};
pub use oauth::{DeviceCodeResponse, OAuthPollResult};
pub use pr::{CreatePullRequest, MergePullRequest, PullRequestInfo, ReviewEvent};
pub use proxy::CorporateProxyConfig;
pub use repos::sanitize_readme_html;
pub use squash_reconciler::SquashReconciler;
pub use ssh_keys::{GitHubSshKeyRecord, RegisterSshKeyRequest};
