#![forbid(unsafe_code)]
//! Lightweight pre-flight connectivity probe for remote Git endpoints.

use git2::{Cred, CredentialType, Direction, Remote, RemoteCallbacks};
use gitupload_core::models::{RemoteEndpoint, RemoteHealthReport, RemoteHealthStatus};
use gitupload_security::{sanitize_log_message, RemoteVaultResolver};
use secrecy::ExposeSecret;
use std::time::{Duration, Instant};

/// Probes Git remote endpoints using a fast `ls-remote`-style handshake within a bounded timeout.
pub struct RemoteHealthChecker;

impl RemoteHealthChecker {
    /// Checks connectivity and authentication readiness for `endpoint` without fetching objects.
    pub async fn probe(
        endpoint: &RemoteEndpoint,
        vault: &RemoteVaultResolver,
    ) -> RemoteHealthReport {
        let started = Instant::now();
        let timeout_secs = endpoint.timeout_seconds.clamp(1, 30) as u64;
        let url = endpoint.url.clone();
        let remote_name = endpoint.name.clone();
        let secret = if endpoint.auth_account.trim().is_empty() {
            None
        } else {
            vault
                .resolve_remote_token(&endpoint.url, &endpoint.auth_account)
                .ok()
                .flatten()
        };

        let fut = tokio::task::spawn_blocking(move || {
            let mut remote = Remote::create_detached(url.as_str())?;
            let mut callbacks = RemoteCallbacks::new();
            let tok_owned = secret.map(|s| s.expose_secret().to_string());
            callbacks.credentials(move |_url, user, allowed| {
                if allowed.contains(CredentialType::SSH_KEY) {
                    return Cred::ssh_key_from_agent(user.unwrap_or("git"));
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
                    "Missing credentials for remote probe",
                ))
            });
            remote.connect_auth(Direction::Fetch, Some(callbacks), None)?;
            let _ = remote.disconnect();
            Ok::<(), git2::Error>(())
        });

        match tokio::time::timeout(Duration::from_secs(timeout_secs), fut).await {
            Ok(Ok(Ok(()))) => RemoteHealthReport {
                remote_name,
                status: RemoteHealthStatus::Healthy,
                latency_ms: started.elapsed().as_millis() as u64,
                message: "Handshake succeeded".to_string(),
            },
            Ok(Ok(Err(err))) => {
                let msg = sanitize_log_message(&err.to_string());
                let lower = msg.to_ascii_lowercase();
                let status = if lower.contains("401")
                    || lower.contains("403")
                    || lower.contains("auth")
                    || lower.contains("credential")
                    || lower.contains("permission")
                {
                    RemoteHealthStatus::AuthFailed
                } else {
                    RemoteHealthStatus::Unreachable
                };
                RemoteHealthReport {
                    remote_name,
                    status,
                    latency_ms: started.elapsed().as_millis() as u64,
                    message: msg,
                }
            }
            Ok(Err(join_err)) => RemoteHealthReport {
                remote_name,
                status: RemoteHealthStatus::Unreachable,
                latency_ms: started.elapsed().as_millis() as u64,
                message: sanitize_log_message(&join_err.to_string()),
            },
            Err(_elapsed) => RemoteHealthReport {
                remote_name,
                status: RemoteHealthStatus::Timeout,
                latency_ms: started.elapsed().as_millis() as u64,
                message: format!("Probe timed out after {timeout_secs}s"),
            },
        }
    }
}
