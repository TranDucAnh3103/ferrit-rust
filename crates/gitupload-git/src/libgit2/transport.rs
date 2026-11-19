//! Credential-safe `libgit2` network transport with cooperative cancellation callbacks.

use crate::coordinator::SafePushMode;
use git2::build::RepoBuilder;
use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::{ExposeSecret, SecretString};
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// In-process `libgit2` push, fetch, and clone transport.
pub struct LibGit2Transport;

impl LibGit2Transport {
    /// Ensures `origin` remote points to a clean, token-free URL.
    pub fn set_clean_remote(
        repo: &Repository,
        remote_name: &str,
        url: &str,
    ) -> GitUploadResult<()> {
        if repo.find_remote(remote_name).is_ok() {
            repo.remote_set_url(remote_name, url).map_err(|e| {
                GitUploadError::Git(format!("Failed updating remote '{remote_name}': {e}"))
            })?;
        } else {
            repo.remote(remote_name, url).map_err(|e| {
                GitUploadError::Git(format!("Failed creating remote '{remote_name}': {e}"))
            })?;
        }
        Ok(())
    }

    /// Clones a remote repository using in-memory `SecretString` credentials (never embedded in URL).
    pub fn clone_repo<F>(
        url: &str,
        dest: &Path,
        token: Option<&SecretString>,
        cancel: &CancellationToken,
        mut on_progress: F,
    ) -> GitUploadResult<()>
    where
        F: FnMut(u32, &str),
    {
        if cancel.is_cancelled() {
            return Err(GitUploadError::Cancelled);
        }

        let mut callbacks = RemoteCallbacks::new();
        if let Some(secret) = token {
            let tok_owned = secret.expose_secret().to_string();
            callbacks.credentials(move |_url, _username_from_url, _allowed| {
                Cred::userpass_plaintext("x-access-token", &tok_owned)
            });
        }

        let cancel_transfer = cancel.clone();
        callbacks.transfer_progress(move |stats| {
            if cancel_transfer.is_cancelled() {
                return false;
            }
            let total = stats.total_objects().max(1);
            let received = stats.received_objects();
            let pct = ((received as f64 / total as f64) * 100.0).min(100.0) as u32;
            on_progress(pct, "Receiving objects");
            true
        });

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        let cloned = builder.clone(url, dest).map_err(|e| {
            if cancel.is_cancelled() {
                GitUploadError::Cancelled
            } else {
                GitUploadError::Git(format!("Clone failed: {e}"))
            }
        })?;

        // If remote HEAD pointed to an unborn ref while refs/remotes/origin/main exists, check out main.
        if cloned.head().is_err() {
            if let Ok(remote_main) = cloned.find_reference("refs/remotes/origin/main") {
                if let Ok(commit) = remote_main.peel_to_commit() {
                    let _ = cloned.branch("main", &commit, false);
                    let _ = cloned.set_head("refs/heads/main");
                    let _ = cloned.checkout_head(Some(git2::build::CheckoutBuilder::new().force()));
                }
            }
        }

        Ok(())
    }

    /// Queries the live commit OID of `refs/heads/{branch}` on `remote_name` without mutating
    /// local `refs/remotes/*` tracking branches (used for atomic `--force-with-lease` validation).
    pub fn ls_remote_branch_oid(
        repo_path: &Path,
        remote_name: &str,
        branch: &str,
        token: Option<&SecretString>,
        cancel: &CancellationToken,
    ) -> GitUploadResult<Option<git2::Oid>> {
        if cancel.is_cancelled() {
            return Err(GitUploadError::Cancelled);
        }

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let configured_remote = repo
            .find_remote(remote_name)
            .map_err(|e| GitUploadError::Git(format!("Remote '{remote_name}' not found: {e}")))?;
        let remote_url = configured_remote
            .url()
            .ok_or_else(|| GitUploadError::Git(format!("Remote '{remote_name}' has no URL")))?
            .to_string();
        drop(configured_remote);

        // Use an anonymous transient remote with NO saved `.git/config` refspecs so probing
        // never mutates `refs/remotes/<remote>/*` tracking branches or FETCH_HEAD side effects.
        let mut remote = repo
            .remote_anonymous(&remote_url)
            .map_err(|e| GitUploadError::Git(format!("Failed opening anonymous remote: {e}")))?;

        let mut callbacks = RemoteCallbacks::new();
        if let Some(secret) = token {
            let tok_owned = secret.expose_secret().to_string();
            callbacks.credentials(move |_url, _username, _allowed| {
                Cred::userpass_plaintext("x-access-token", &tok_owned)
            });
        }

        let cancel_cb = cancel.clone();
        callbacks.transfer_progress(move |_| !cancel_cb.is_cancelled());

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        // Fetch exclusively into an ephemeral probe ref so local `refs/remotes/*` is never mutated
        // on rejected pushes and we avoid `git2::RemoteConnection::list()` null-slice UB on empty remotes.
        let probe_ref = format!("refs/gitupload-lease/{remote_name}/{branch}");
        if let Ok(mut existing) = repo.find_reference(&probe_ref) {
            let _ = existing.delete();
        }

        let refspec = format!("+refs/heads/{branch}:{probe_ref}");
        if remote
            .fetch(&[&refspec], Some(&mut fetch_opts), None)
            .is_err()
        {
            return Ok(None);
        }

        let found = if let Ok(mut reference) = repo.find_reference(&probe_ref) {
            let oid = reference.target();
            let _ = reference.delete();
            oid
        } else {
            None
        };
        Ok(found)
    }

    /// Fetches remote tracking refs from `remote_name` with cancellation support.
    pub fn fetch_remote(
        repo_path: &Path,
        remote_name: &str,
        token: Option<&SecretString>,
        cancel: &CancellationToken,
    ) -> GitUploadResult<()> {
        if cancel.is_cancelled() {
            return Err(GitUploadError::Cancelled);
        }

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| GitUploadError::Git(format!("Remote '{remote_name}' not found: {e}")))?;

        let mut callbacks = RemoteCallbacks::new();
        if let Some(secret) = token {
            let tok_owned = secret.expose_secret().to_string();
            callbacks.credentials(move |_url, _username, _allowed| {
                Cred::userpass_plaintext("x-access-token", &tok_owned)
            });
        }

        let cancel_cb = cancel.clone();
        callbacks.transfer_progress(move |_| !cancel_cb.is_cancelled());

        let mut fetch_opts = FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        let refspec = format!("+refs/heads/*:refs/remotes/{remote_name}/*");
        remote
            .fetch(&[&refspec], Some(&mut fetch_opts), None)
            .map_err(|e| {
                if cancel.is_cancelled() {
                    GitUploadError::Cancelled
                } else {
                    GitUploadError::Git(format!("Fetch failed: {e}"))
                }
            })?;

        Ok(())
    }

    /// Pushes `branch` to `remote_name` respecting `SafePushMode` (`StrictSafe`, `ForceWithLease`, `ForceUnsafe`).
    pub fn push_branch<F>(
        repo_path: &Path,
        remote_name: &str,
        branch: &str,
        token: Option<&SecretString>,
        mode: SafePushMode,
        cancel: &CancellationToken,
        mut on_progress: F,
    ) -> GitUploadResult<()>
    where
        F: FnMut(u32, &str),
    {
        if cancel.is_cancelled() {
            return Err(GitUploadError::Cancelled);
        }

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;
        let mut remote = repo
            .find_remote(remote_name)
            .map_err(|e| GitUploadError::Git(format!("Remote '{remote_name}' not found: {e}")))?;

        let mut callbacks = RemoteCallbacks::new();
        if let Some(secret) = token {
            let tok_owned = secret.expose_secret().to_string();
            callbacks.credentials(move |_url, _username, _allowed| {
                Cred::userpass_plaintext("x-access-token", &tok_owned)
            });
        }

        let cancel_transfer = cancel.clone();
        callbacks.push_transfer_progress(move |current, total, _bytes| {
            if !cancel_transfer.is_cancelled() && total > 0 {
                let pct = ((current as f64 / total as f64) * 100.0).min(100.0) as u32;
                on_progress(pct, "Uploading Git pack objects");
            }
        });

        let push_err = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let push_err_ref = push_err.clone();
        callbacks.push_update_reference(move |refname, status| {
            if let Some(msg) = status {
                if let Ok(mut guard) = push_err_ref.lock() {
                    *guard = Some(format!("Remote rejected {refname}: {msg}"));
                }
            }
            Ok(())
        });

        let mut push_opts = PushOptions::new();
        push_opts.remote_callbacks(callbacks);

        let refspec = match mode {
            SafePushMode::StrictSafe => format!("refs/heads/{branch}:refs/heads/{branch}"),
            SafePushMode::ForceWithLease | SafePushMode::ForceUnsafe => {
                format!("+refs/heads/{branch}:refs/heads/{branch}")
            }
        };

        remote
            .push(&[&refspec], Some(&mut push_opts))
            .map_err(|e| {
                if cancel.is_cancelled() {
                    GitUploadError::Cancelled
                } else {
                    GitUploadError::Git(format!("Push failed: {e}"))
                }
            })?;

        if let Ok(mut guard) = push_err.lock() {
            if let Some(msg) = guard.take() {
                return Err(GitUploadError::Git(msg));
            }
        }

        Ok(())
    }
}
