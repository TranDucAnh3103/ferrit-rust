#![forbid(unsafe_code)]
//! Autonomous background fetch & safe fast-forward synchronization engine.

use git2::{Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository};
use gitupload_core::errors::{GitError, GitResult};
use gitupload_core::models::AutoSyncOutcome;
use secrecy::{ExposeSecret, SecretString};
use tracing::info;

/// Performs silent background fetches and safe fast-forwards when the worktree is clean.
pub struct AutoSyncEngine;

impl AutoSyncEngine {
    /// Fetches `branch_name` from `remote_name` using optional `token` and fast-forwards cleanly if un-diverged.
    pub fn sync_repository(
        repo_path: &str,
        remote_name: &str,
        branch_name: &str,
        token: Option<&SecretString>,
    ) -> GitResult<AutoSyncOutcome> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitError::RepoOpen(repo_path.to_string(), e.to_string()))?;

        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| GitError::Engine(format!("Remote '{remote_name}' not found: {e}")))?;

        let mut callbacks = RemoteCallbacks::new();
        let tok_owned = token.map(|s| s.expose_secret().to_string());
        callbacks.credentials(move |_url, user, allowed| {
            if allowed.contains(CredentialType::SSH_KEY) {
                return Cred::ssh_key_from_agent(user.unwrap_or("git"));
            }
            if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
                if let Some(ref tok) = tok_owned {
                    return Cred::userpass_plaintext("x-access-token", tok);
                }
            }
            if allowed.contains(CredentialType::DEFAULT) {
                return Cred::default();
            }
            Err(git2::Error::from_str("No credentials for background fetch"))
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let refspec = format!("+refs/heads/{branch_name}:refs/remotes/{remote_name}/{branch_name}");
        remote
            .fetch(&[&refspec], Some(&mut fetch_opts), None)
            .map_err(|e| GitError::Engine(format!("Background fetch failed: {e}")))?;

        let local_ref = format!("refs/heads/{branch_name}");
        let remote_ref = format!("refs/remotes/{remote_name}/{branch_name}");

        let local_oid = repo
            .refname_to_id(&local_ref)
            .map_err(|e| GitError::Engine(format!("Local ref '{local_ref}' error: {e}")))?;
        let remote_oid = repo
            .refname_to_id(&remote_ref)
            .map_err(|e| GitError::Engine(format!("Remote ref '{remote_ref}' error: {e}")))?;

        let (ahead, behind) = repo
            .graph_ahead_behind(local_oid, remote_oid)
            .map_err(|e| GitError::Engine(e.to_string()))?;

        if ahead == 0 && behind == 0 {
            return Ok(AutoSyncOutcome::UpToDate);
        }

        if ahead == 0 && behind > 0 {
            let statuses = repo
                .statuses(None)
                .map_err(|e| GitError::Engine(e.to_string()))?;
            if statuses.is_empty() {
                let remote_commit = repo
                    .find_commit(remote_oid)
                    .map_err(|e| GitError::Engine(e.to_string()))?;
                repo.checkout_tree(remote_commit.as_object(), None)
                    .map_err(|e| GitError::Engine(e.to_string()))?;
                let mut reference = repo
                    .find_reference(&local_ref)
                    .map_err(|e| GitError::Engine(e.to_string()))?;
                reference
                    .set_target(remote_oid, "Fast-forward autonomous sync")
                    .map_err(|e| GitError::Engine(e.to_string()))?;
                info!("Autonomous fast-forward applied: {behind} commits pulled");
                return Ok(AutoSyncOutcome::FastForwarded {
                    commits_pulled: behind,
                });
            }
        }

        Ok(AutoSyncOutcome::Diverged { ahead, behind })
    }
}
