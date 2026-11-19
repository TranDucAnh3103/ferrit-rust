//! GitHub API cross-referencing to detect pull requests merged via squash & merge.

use crate::client::GitHubClient;
use gitupload_core::errors::GitHubResult;
use gitupload_core::models::{BranchMergeStatus, StaleBranchInfo};

/// Queries GitHub PR metadata to detect squashed merges that differ by commit OID locally.
pub struct SquashReconciler;

impl SquashReconciler {
    /// Updates any non-`FullyMerged`, unprotected candidate whose PR on `owner/repo` was merged.
    pub async fn reconcile_squashed_branches(
        client: &GitHubClient,
        owner: &str,
        repo: &str,
        branches: &mut [StaleBranchInfo],
    ) -> GitHubResult<()> {
        for b in branches.iter_mut() {
            if b.merge_status != BranchMergeStatus::FullyMerged && !b.is_protected {
                let url = format!(
                    "{}/repos/{}/{}/pulls?head={}:{}&state=closed",
                    client.base_url, owner, repo, owner, b.name
                );
                if let Ok(prs) = client.get::<Vec<serde_json::Value>>(&url).await {
                    for pr in prs {
                        if !pr["merged_at"].is_null() {
                            b.merge_status = BranchMergeStatus::SquashedAndMerged;
                            b.associated_pr_number = pr["number"].as_u64();
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
