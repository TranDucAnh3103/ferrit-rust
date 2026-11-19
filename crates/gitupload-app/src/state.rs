//! Managed application state shared across Tauri IPC commands.

use crate::webhooks::WebhookDispatcher;
use gitupload_ai::{CloudAIOrchestrator, CloudAiService};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_ghcli::GhLifecycleManager;
use gitupload_git::{GitCoordinator, GitEngine};
use gitupload_github::GitHubClient;
use gitupload_plugins::{PluginInstaller, PluginRunner};
use gitupload_security::{
    KeyringManager, NativeSshAgent, PlatformVault, RemoteVaultResolver, SshVault,
};
use gitupload_storage::StoragePool;
use secrecy::SecretString;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Central application state injected into Tauri commands via `tauri::State<'_, AppState>`.
pub struct AppState {
    /// OS Credential Vault manager.
    pub keyring: KeyringManager,
    /// Cross-platform OS Credential Vault abstraction.
    pub platform_vault: Arc<PlatformVault>,
    /// Per-remote credential resolver backed by Windows Credential Manager.
    pub remote_vault: Arc<RemoteVaultResolver>,
    /// DPAPI-backed SSH private-key vault.
    pub ssh_vault: Arc<SshVault>,
    /// Pure-Rust Windows Named Pipe SSH Agent daemon.
    pub ssh_agent: Arc<NativeSshAgent>,
    /// Embedded SQLite WAL persistence pool.
    pub storage: StoragePool,
    /// High-level Git workflow coordinator (push, single-file isolation, re-push).
    pub git: Arc<GitCoordinator>,
    /// In-process `libgit2` engine.
    pub git_engine: Arc<GitEngine>,
    /// GitHub CLI (`gh`) sandbox lifecycle manager.
    pub gh_cli: GhLifecycleManager,
    /// Base connection-pooled GitHub HTTP client.
    pub github: Arc<GitHubClient>,
    /// Pure Cloud AI orchestrator.
    pub ai: Arc<CloudAIOrchestrator>,
    /// Streaming Cloud AI HTTP client (used by 3-way AI merge assistant).
    pub ai_client: CloudAiService,
    /// Multi-channel event webhook dispatcher.
    pub webhooks: Arc<WebhookDispatcher>,
    /// Filesystem directory where WASM plugins are installed.
    pub plugins_dir: std::path::PathBuf,
    /// WebAssembly plugin runner.
    pub plugin_runner: Arc<PluginRunner>,
    /// WebAssembly plugin installer.
    pub plugin_installer: Arc<PluginInstaller>,
    /// Active cancellation token for long-running push/clone jobs.
    pub active_job_cancel: Arc<Mutex<Option<CancellationToken>>>,
    /// Active cancellation token for debounced directory filesystem watcher.
    pub watcher_cancel: Arc<Mutex<Option<CancellationToken>>>,
    /// Active cancellation token for background scheduled sync daemon.
    pub sync_daemon_cancel: Arc<Mutex<Option<CancellationToken>>>,
}

impl AppState {
    /// Constructs a new `AppState` wired to an initialized `StoragePool`.
    pub fn new(storage: StoragePool) -> Self {
        let shared_keyring = Arc::new(KeyringManager::new());
        let platform_vault = Arc::new(PlatformVault::system());
        let remote_vault = Arc::new(RemoteVaultResolver::with_keyring(Arc::clone(
            &shared_keyring,
        )));
        let ssh_vault = Arc::new(SshVault::with_keyring(shared_keyring));
        let ssh_agent = Arc::new(NativeSshAgent::default_windows_pipe(Arc::clone(&ssh_vault)));

        let webhooks = Arc::new(WebhookDispatcher::new(
            storage.clone(),
            platform_vault.clone(),
        ));

        let proj_dirs = directories::ProjectDirs::from("com", "stephen", "GitUpload");
        let plugins_dir = proj_dirs
            .as_ref()
            .map(|pd| pd.data_dir().join("plugins"))
            .unwrap_or_else(|| std::path::PathBuf::from("plugins"));
        let _ = std::fs::create_dir_all(&plugins_dir);

        let plugin_runner =
            Arc::new(PluginRunner::new().expect("Failed to initialize WASM PluginEngine"));
        let plugin_installer = Arc::new(PluginInstaller::new(plugins_dir.clone()));

        Self {
            keyring: KeyringManager::new(),
            platform_vault,
            remote_vault,
            ssh_vault,
            ssh_agent,
            storage,
            git: Arc::new(GitCoordinator::new()),
            git_engine: Arc::new(GitEngine::new()),
            gh_cli: GhLifecycleManager::new(),
            github: Arc::new(GitHubClient::default()),
            ai: Arc::new(CloudAIOrchestrator::new()),
            ai_client: CloudAiService::new(),
            webhooks,
            plugins_dir,
            plugin_runner,
            plugin_installer,
            active_job_cancel: Arc::new(Mutex::new(None)),
            watcher_cancel: Arc::new(Mutex::new(None)),
            sync_daemon_cancel: Arc::new(Mutex::new(None)),
        }
    }

    /// Resolves the GitHub PAT for `account_id` from OS Keyring and constructs a credentialed `GitHubClient`.
    pub fn github_client_for_account(&self, account_id: &str) -> GitUploadResult<GitHubClient> {
        let key = KeyringManager::github_account_key(account_id);
        let token = self.keyring.get_token(&key)?.ok_or_else(|| {
            GitUploadError::Vault(
                "Chưa có GitHub Token trong Windows Credential Manager. Vui lòng nhập token hoặc Sync từ `gh` trong Settings.".into(),
            )
        })?;
        Ok(self.github.with_token(Some(token)))
    }

    /// Retrieves optional `SecretString` token for `account_id` from OS Keyring.
    pub fn get_github_token(&self, account_id: &str) -> GitUploadResult<Option<SecretString>> {
        let key = KeyringManager::github_account_key(account_id);
        self.keyring.get_token(&key)
    }

    /// Retrieves optional Cloud AI API key for `provider` from OS Keyring with xKiro default fallback.
    pub fn get_ai_key(&self, provider: &str) -> GitUploadResult<Option<SecretString>> {
        let key = KeyringManager::ai_provider_key(provider);
        if let Ok(Some(tok)) = self.keyring.get_token(&key) {
            return Ok(Some(tok));
        }
        if provider.eq_ignore_ascii_case("xkiro") || provider.eq_ignore_ascii_case("default") {
            return Ok(Some(SecretString::from(
                "sk-xt-42685b582101d940e68aeab65664ebb6e546d047604010c4".to_string(),
            )));
        }
        Ok(None)
    }
}
