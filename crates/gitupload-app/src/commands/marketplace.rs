use crate::marketplace::MarketplaceCatalog;
use crate::state::AppState;
use chrono::Utc;
use gitupload_core::errors::IpcErrorPayload;
use gitupload_core::marketplace::{MarketplaceIndex, MarketplacePluginEntry};
use gitupload_core::plugin_manifest::{PluginCapabilities, UserPluginRecord};
use gitupload_storage::PluginsRepository;
use tauri::State;

#[tauri::command]
pub async fn marketplace_fetch_catalog(
    _state: State<'_, AppState>,
    custom_url: Option<String>,
) -> Result<MarketplaceIndex, IpcErrorPayload> {
    let catalog = MarketplaceCatalog::new();
    let (index, _is_offline) = catalog.fetch_catalog(custom_url.as_deref()).await;
    Ok(index)
}

#[tauri::command]
pub async fn marketplace_install_plugin(
    state: State<'_, AppState>,
    entry: MarketplacePluginEntry,
) -> Result<UserPluginRecord, IpcErrorPayload> {
    // 1. Download bundle
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| IpcErrorPayload {
            code: "DOWNLOAD_ERROR".into(),
            message: e.to_string(),
            recoverable: false,
        })?;

    let res = client
        .get(&entry.download_url)
        .send()
        .await
        .map_err(|e| IpcErrorPayload {
            code: "DOWNLOAD_FAILED".into(),
            message: format!("Failed to download plugin package: {e}"),
            recoverable: true,
        })?;

    if !res.status().is_success() {
        return Err(IpcErrorPayload {
            code: "DOWNLOAD_HTTP_ERROR".into(),
            message: format!("Plugin server returned HTTP status {}", res.status()),
            recoverable: false,
        });
    }

    let bundle_bytes = res.bytes().await.map_err(|e| IpcErrorPayload {
        code: "DOWNLOAD_READ_ERROR".into(),
        message: e.to_string(),
        recoverable: true,
    })?;

    // 2. Install through atomic installer with SHA256 & Ed25519 verification
    state
        .plugin_installer
        .install(
            &entry.id,
            &bundle_bytes,
            &entry.sha256,
            &entry.ed25519_signature,
            &entry.publisher_pubkey,
        )
        .map_err(|e| IpcErrorPayload {
            code: "INSTALL_VERIFY_ERROR".into(),
            message: e.to_string(),
            recoverable: false,
        })?;

    // 3. Register in database
    let caps = PluginCapabilities {
        git_read: entry.capabilities.iter().any(|c| c == "git_read"),
        git_write: entry.capabilities.iter().any(|c| c == "git_write"),
        commit_hook: entry.capabilities.iter().any(|c| c == "commit_hook"),
        ai_prompt_filter: entry.capabilities.iter().any(|c| c == "ai_prompt_filter"),
        network_access: entry.capabilities.iter().any(|c| c == "network_access"),
        max_memory_mb: 16,
    };

    let record = UserPluginRecord {
        id: entry.id.clone(),
        name: entry.name.clone(),
        version: entry.version.clone(),
        author: entry.author.clone(),
        entrypoint: "plugin.wasm".into(),
        capabilities: caps,
        is_enabled: true,
        installed_at: Utc::now(),
    };

    let repo = PluginsRepository::new(state.storage.clone());
    repo.register(&record).map_err(IpcErrorPayload::from)?;

    Ok(record)
}
