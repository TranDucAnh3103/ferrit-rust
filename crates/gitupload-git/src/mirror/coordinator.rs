#![forbid(unsafe_code)]
//! Concurrent multi-remote push/fetch coordinator backed by `tokio::task::JoinSet` and isolated timeouts.

use crate::coordinator::SafePushMode;
use git2::{Cred, CredentialType, PushOptions, RemoteCallbacks, Repository};
use gitupload_core::errors::GitUploadError;
use gitupload_core::models::{
    MultiRemoteProgressPayload, RemoteEndpoint, RemotePushResult, SyncStatus,
};
use gitupload_security::{sanitize_log_message, RemoteVaultResolver};
use secrecy::ExposeSecret;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

/// Coordinates simultaneous pushes across multiple configured Git remotes with per-remote failure & timeout isolation.
pub struct MultiRemoteCoordinator;

impl MultiRemoteCoordinator {
    /// Pushes `branch` concurrently to all enabled remotes in `endpoints`.
    /// Disabled remotes produce `SyncStatus::Skipped` immediately without blocking peers.
    pub async fn push_all(
        repo_path: &Path,
        branch: &str,
        endpoints: &[RemoteEndpoint],
        mode: SafePushMode,
        vault: &RemoteVaultResolver,
        progress_tx: Option<mpsc::Sender<MultiRemoteProgressPayload>>,
        cancel: CancellationToken,
    ) -> Vec<RemotePushResult> {
        let mut results: Vec<RemotePushResult> = Vec::with_capacity(endpoints.len());
        let mut progress_map: BTreeMap<String, u32> = BTreeMap::new();
        let mut join_set: JoinSet<RemotePushResult> = JoinSet::new();

        for ep in endpoints {
            if !ep.is_enabled {
                progress_map.insert(ep.name.clone(), 100);
                results.push(RemotePushResult {
                    remote_name: ep.name.clone(),
                    is_success: true,
                    status: SyncStatus::Skipped,
                    bytes_transferred: 0,
                    error_message: None,
                    duration_ms: 0,
                });
                continue;
            }

            progress_map.insert(ep.name.clone(), 0);
            let ep_clone = ep.clone();
            let repo_buf: PathBuf = repo_path.to_path_buf();
            let branch_str = branch.to_string();
            let task_cancel = cancel.child_token();

            // Resolve token ahead of blocking Git task without logging it
            let resolved_token = if ep_clone.auth_account.trim().is_empty() {
                None
            } else {
                vault
                    .resolve_remote_token(&ep_clone.url, &ep_clone.auth_account)
                    .ok()
                    .flatten()
            };

            join_set.spawn(async move {
                let started = Instant::now();
                let timeout_secs = ep_clone.timeout_seconds.max(1) as u64;
                let remote_name = ep_clone.name.clone();
                let bytes_counter = Arc::new(AtomicUsize::new(0));
                let bytes_ref = Arc::clone(&bytes_counter);

                let ep_for_blocking = ep_clone.clone();
                let cancel_for_blocking = task_cancel.clone();
                let push_future = tokio::task::spawn_blocking(move || {
                    push_single_remote(
                        &repo_buf,
                        &ep_for_blocking,
                        &branch_str,
                        resolved_token,
                        mode,
                        cancel_for_blocking,
                        bytes_ref,
                    )
                });

                let timeout_res =
                    tokio::time::timeout(Duration::from_secs(timeout_secs), push_future).await;
                let duration_ms = started.elapsed().as_millis() as u64;
                let transferred = bytes_counter.load(Ordering::Relaxed);

                match timeout_res {
                    Ok(Ok(Ok(()))) => RemotePushResult {
                        remote_name,
                        is_success: true,
                        status: SyncStatus::Success,
                        bytes_transferred: transferred,
                        error_message: None,
                        duration_ms,
                    },
                    Ok(Ok(Err(err))) => RemotePushResult {
                        remote_name,
                        is_success: false,
                        status: SyncStatus::Failed,
                        bytes_transferred: transferred,
                        error_message: Some(sanitize_log_message(&err.to_string())),
                        duration_ms,
                    },
                    Ok(Err(join_err)) => RemotePushResult {
                        remote_name,
                        is_success: false,
                        status: SyncStatus::Failed,
                        bytes_transferred: transferred,
                        error_message: Some(sanitize_log_message(&format!(
                            "Remote task panicked: {join_err}"
                        ))),
                        duration_ms,
                    },
                    Err(_elapsed) => {
                        task_cancel.cancel();
                        RemotePushResult {
                            remote_name,
                            is_success: false,
                            status: SyncStatus::TimedOut,
                            bytes_transferred: transferred,
                            error_message: Some(format!(
                                "Remote push timed out after {timeout_secs}s"
                            )),
                            duration_ms,
                        }
                    }
                }
            });
        }

        if let Some(tx) = &progress_tx {
            let overall = average_pct(&progress_map);
            let _ = tx.try_send(MultiRemoteProgressPayload {
                remotes: progress_map.clone(),
                overall_percentage: overall,
            });
        }

        while let Some(joined) = join_set.join_next().await {
            if let Ok(res) = joined {
                progress_map.insert(res.remote_name.clone(), 100);
                results.push(res);
                if let Some(tx) = &progress_tx {
                    let overall = average_pct(&progress_map);
                    let _ = tx.try_send(MultiRemoteProgressPayload {
                        remotes: progress_map.clone(),
                        overall_percentage: overall,
                    });
                }
            }
        }

        // Preserve endpoint input ordering deterministically
        let order: BTreeMap<String, usize> = endpoints
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.clone(), i))
            .collect();
        results.sort_by_key(|r| order.get(&r.remote_name).copied().unwrap_or(usize::MAX));
        results
    }
}

fn average_pct(map: &BTreeMap<String, u32>) -> u32 {
    if map.is_empty() {
        return 100;
    }
    let sum: u32 = map.values().copied().sum();
    sum / (map.len() as u32)
}

fn push_single_remote(
    repo_path: &Path,
    endpoint: &RemoteEndpoint,
    branch: &str,
    token: Option<secrecy::SecretString>,
    mode: SafePushMode,
    cancel: CancellationToken,
    bytes_transferred: Arc<AtomicUsize>,
) -> Result<(), GitUploadError> {
    if cancel.is_cancelled() {
        return Err(GitUploadError::Cancelled);
    }

    let repo = Repository::open(repo_path)
        .map_err(|e| GitUploadError::Git(format!("Failed opening repository: {e}")))?;

    // Use lock-free anonymous remote to eliminate `.git/config.lock` contention across parallel tasks
    let mut remote = repo.remote_anonymous(&endpoint.url).map_err(|e| {
        GitUploadError::Git(format!(
            "Failed initializing anonymous remote '{}': {e}",
            endpoint.name
        ))
    })?;

    let mut callbacks = RemoteCallbacks::new();
    let tok_owned = token.map(|s| s.expose_secret().to_string());
    callbacks.credentials(move |_url, username_from_url, allowed| {
        if allowed.contains(CredentialType::SSH_KEY) {
            let user = username_from_url.unwrap_or("git");
            return Cred::ssh_key_from_agent(user);
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let Some(tok) = &tok_owned {
                return Cred::userpass_plaintext("x-access-token", tok);
            }
        }
        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }
        Err(git2::Error::from_str(
            "No matching credentials or SSH identity available",
        ))
    });

    let bytes_ref = Arc::clone(&bytes_transferred);
    let cancel_transfer = cancel.clone();
    callbacks.push_transfer_progress(move |_current, _total, bytes| {
        bytes_ref.store(bytes, Ordering::Relaxed);
        let _ = cancel_transfer.is_cancelled();
    });

    let push_err = Arc::new(std::sync::Mutex::new(None::<String>));
    let push_err_ref = Arc::clone(&push_err);
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
                GitUploadError::Git(format!("Push to '{}' failed: {e}", endpoint.name))
            }
        })?;

    if let Ok(mut guard) = push_err.lock() {
        if let Some(msg) = guard.take() {
            return Err(GitUploadError::Git(msg));
        }
    }

    Ok(())
}
