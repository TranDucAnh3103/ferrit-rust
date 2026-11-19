//! Pure-safe Tokio Windows Named Pipe SSH Agent daemon (`draft-miller-ssh-agent`).

use crate::ssh::keygen::{decode_openssh_pubkey_blob, sign_challenge_with_secret};
use crate::ssh::protocol::{
    encode_failure, encode_identities_answer, encode_sign_response, frame_message, parse_request,
    SshAgentRequest,
};
use crate::ssh::vault::SshVault;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::SshKeyDescriptor;
use secrecy::SecretString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Default Windows Named Pipe endpoint consumed by native `git.exe`, `ssh.exe`, and libgit2.
pub const DEFAULT_WINDOWS_SSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

/// An unlocked identity held strictly in volatile memory while the SSH agent is running.
#[derive(Clone)]
struct UnlockedIdentity {
    key_id: String,
    title: String,
    public_key_blob: Vec<u8>,
    private_material: SecretString,
}

/// Pure-Rust in-process SSH Agent serving keys from [`SshVault`] over a Windows Named Pipe.
pub struct NativeSshAgent {
    pipe_name: String,
    vault: Arc<SshVault>,
    identities: Arc<RwLock<Vec<UnlockedIdentity>>>,
    running: Arc<AtomicBool>,
    cancel_guard: Mutex<Option<CancellationToken>>,
}

impl NativeSshAgent {
    /// Creates an SSH agent bound to `pipe_name` (for example `\\.\pipe\gitupload-ssh-agent`).
    pub fn new(pipe_name: impl Into<String>, vault: Arc<SshVault>) -> Self {
        Self {
            pipe_name: pipe_name.into(),
            vault,
            identities: Arc::new(RwLock::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            cancel_guard: Mutex::new(None),
        }
    }

    /// Creates an SSH agent bound to the standard OpenSSH Windows pipe `\\.\pipe\openssh-ssh-agent`.
    pub fn default_windows_pipe(vault: Arc<SshVault>) -> Self {
        Self::new(DEFAULT_WINDOWS_SSH_AGENT_PIPE, vault)
    }

    /// Returns the configured Named Pipe path.
    pub fn pipe_name(&self) -> &str {
        &self.pipe_name
    }

    /// Returns `true` when the Named Pipe listener loop is active.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns the number of identities currently unlocked in volatile memory.
    pub fn loaded_key_count(&self) -> usize {
        self.identities.read().map(|g| g.len()).unwrap_or(0)
    }

    /// Reloads the in-memory unlocked identity list from SQLite descriptors + OS Credential Vault.
    pub fn reload_identities(&self, descriptors: &[SshKeyDescriptor]) -> GitUploadResult<usize> {
        let mut unlocked = Vec::with_capacity(descriptors.len());
        for desc in descriptors {
            let Some(secret) = self.vault.load_private_key(&desc.key_id)? else {
                continue;
            };
            let blob = decode_openssh_pubkey_blob(&desc.public_key)?;
            unlocked.push(UnlockedIdentity {
                key_id: desc.key_id.clone(),
                title: desc.title.clone(),
                public_key_blob: blob,
                private_material: secret,
            });
        }
        let count = unlocked.len();
        let mut guard = self
            .identities
            .write()
            .map_err(|e| GitUploadError::Vault(format!("SSH identity lock poisoned: {e}")))?;
        *guard = unlocked;
        Ok(count)
    }

    /// Starts the Named Pipe SSH Agent listener with the provided unlocked key descriptors.
    pub async fn start(&self, descriptors: &[SshKeyDescriptor]) -> GitUploadResult<()> {
        self.reload_identities(descriptors)?;

        let mut token_slot = self.cancel_guard.lock().await;
        if let Some(existing) = token_slot.take() {
            existing.cancel();
            self.running.store(false, Ordering::SeqCst);
        }

        let cancel = CancellationToken::new();
        *token_slot = Some(cancel.clone());
        drop(token_slot);

        self.spawn_listener(cancel).await
    }

    /// Stops the SSH Agent listener and wipes unlocked key material from volatile memory.
    pub async fn stop(&self) {
        let mut token_slot = self.cancel_guard.lock().await;
        if let Some(cancel) = token_slot.take() {
            cancel.cancel();
        }
        self.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.identities.write() {
            guard.clear();
        }
    }

    /// Handles a single raw SSH agent wire frame payload directly in memory.
    pub fn handle_payload(&self, payload: &[u8]) -> Vec<u8> {
        let Ok(req) = parse_request(payload) else {
            return encode_failure();
        };

        match req {
            SshAgentRequest::RequestIdentities => {
                let list: Vec<(Vec<u8>, String)> = self
                    .identities
                    .read()
                    .map(|guard| {
                        guard
                            .iter()
                            .map(|id| {
                                (
                                    id.public_key_blob.clone(),
                                    format!("{} ({})", id.title, id.key_id),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                encode_identities_answer(&list)
            }
            SshAgentRequest::SignRequest {
                key_blob,
                data,
                flags,
            } => {
                let matched_secret = self.identities.read().ok().and_then(|guard| {
                    guard
                        .iter()
                        .find(|id| id.public_key_blob == key_blob)
                        .map(|id| id.private_material.clone())
                });
                let Some(secret) = matched_secret else {
                    return encode_failure();
                };
                match sign_challenge_with_secret(&secret, &data, flags) {
                    Ok(sig_blob) => encode_sign_response(&sig_blob),
                    Err(_) => encode_failure(),
                }
            }
            SshAgentRequest::Unsupported(_) => encode_failure(),
        }
    }

    #[cfg(windows)]
    async fn spawn_listener(&self, cancel: CancellationToken) -> GitUploadResult<()> {
        use tokio::net::windows::named_pipe::ServerOptions;

        let pipe_name = self.pipe_name.clone();
        let initial_server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)
            .map_err(|e| {
                GitUploadError::Vault(format!(
                    "Failed to bind SSH agent Named Pipe '{pipe_name}': {e}"
                ))
            })?;

        self.running.store(true, Ordering::SeqCst);
        let identities = Arc::clone(&self.identities);
        let running = Arc::clone(&self.running);
        let vault = Arc::clone(&self.vault);

        tokio::spawn(async move {
            let mut server = initial_server;
            let helper_agent = NativeSshAgent {
                pipe_name: pipe_name.clone(),
                vault,
                identities,
                running: Arc::clone(&running),
                cancel_guard: Mutex::new(None),
            };

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    accept_res = server.connect() => {
                        match accept_res {
                            Ok(()) => {
                                let connected_pipe = server;
                                match ServerOptions::new().create(&pipe_name) {
                                    Ok(next_server) => server = next_server,
                                    Err(_) => break,
                                }
                                let conn_identities = Arc::clone(&helper_agent.identities);
                                let conn_vault = Arc::clone(&helper_agent.vault);
                                let conn_pipe = pipe_name.clone();
                                let conn_running = Arc::clone(&running);
                                tokio::spawn(async move {
                                    let session_agent = NativeSshAgent {
                                        pipe_name: conn_pipe,
                                        vault: conn_vault,
                                        identities: conn_identities,
                                        running: conn_running,
                                        cancel_guard: Mutex::new(None),
                                    };
                                    let _ = serve_pipe_connection(connected_pipe, &session_agent).await;
                                });
                            }
                            Err(_) => break,
                        }
                    }
                }
            }

            running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    #[cfg(not(windows))]
    async fn spawn_listener(&self, _cancel: CancellationToken) -> GitUploadResult<()> {
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }
}

async fn serve_pipe_connection<S>(mut stream: S, agent: &NativeSshAgent) -> std::io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut len_buf = [0u8; 4];
    loop {
        if stream.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let msg_len = u32::from_be_bytes(len_buf) as usize;
        if msg_len == 0 || msg_len > 256 * 1024 {
            break;
        }
        let mut payload = vec![0u8; msg_len];
        if stream.read_exact(&mut payload).await.is_err() {
            break;
        }
        let response_payload = agent.handle_payload(&payload);
        let framed = frame_message(&response_payload);
        stream.write_all(&framed).await?;
        stream.flush().await?;
    }
    Ok(())
}

/// Sends a single framed SSH Agent request over a Windows Named Pipe and reads the raw response payload.
#[cfg(windows)]
pub async fn send_named_pipe_request(
    pipe_name: &str,
    request_payload: &[u8],
) -> GitUploadResult<Vec<u8>> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let mut client = ClientOptions::new().open(pipe_name).map_err(|e| {
        GitUploadError::Vault(format!(
            "Failed to connect to SSH agent Named Pipe '{pipe_name}': {e}"
        ))
    })?;

    let framed = frame_message(request_payload);
    client
        .write_all(&framed)
        .await
        .map_err(|e| GitUploadError::Vault(format!("SSH agent write failed: {e}")))?;
    client
        .flush()
        .await
        .map_err(|e| GitUploadError::Vault(format!("SSH agent flush failed: {e}")))?;

    let mut len_buf = [0u8; 4];
    client.read_exact(&mut len_buf).await.map_err(|e| {
        GitUploadError::Vault(format!("SSH agent response header read failed: {e}"))
    })?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;
    if resp_len == 0 || resp_len > 256 * 1024 {
        return Err(GitUploadError::Vault(
            "Invalid SSH agent response length".to_string(),
        ));
    }
    let mut resp = vec![0u8; resp_len];
    client
        .read_exact(&mut resp)
        .await
        .map_err(|e| GitUploadError::Vault(format!("SSH agent response body read failed: {e}")))?;
    Ok(resp)
}
