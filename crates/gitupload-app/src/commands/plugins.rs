use crate::state::AppState;
use gitupload_core::errors::IpcErrorPayload;
use gitupload_core::plugin_manifest::UserPluginRecord;
use gitupload_storage::PluginsRepository;
use tauri::State;

#[tauri::command]
pub async fn plugin_list_installed(
    state: State<'_, AppState>,
) -> Result<Vec<UserPluginRecord>, IpcErrorPayload> {
    let repo = PluginsRepository::new(state.storage.clone());
    repo.list_installed().map_err(IpcErrorPayload::from)
}

#[tauri::command]
pub async fn plugin_toggle_active(
    state: State<'_, AppState>,
    plugin_id: String,
    enabled: bool,
) -> Result<(), IpcErrorPayload> {
    let repo = PluginsRepository::new(state.storage.clone());
    repo.set_enabled(&plugin_id, enabled)
        .map_err(IpcErrorPayload::from)
}

#[tauri::command]
pub async fn plugin_uninstall(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), IpcErrorPayload> {
    state
        .plugin_installer
        .uninstall(&plugin_id)
        .map_err(|e| IpcErrorPayload {
            code: "PLUGIN_UNINSTALL_FAILED".into(),
            message: e.to_string(),
            recoverable: false,
        })?;

    let repo = PluginsRepository::new(state.storage.clone());
    let _ = repo.delete(&plugin_id);
    Ok(())
}

#[tauri::command]
pub async fn plugin_execute_commit_hook(
    state: State<'_, AppState>,
    commit_msg: String,
    branch: String,
    staged_files: Vec<String>,
) -> Result<(), IpcErrorPayload> {
    let repo = PluginsRepository::new(state.storage.clone());
    let plugins = repo.list_installed().map_err(IpcErrorPayload::from)?;

    for plugin in plugins {
        if !plugin.is_enabled || !plugin.capabilities.commit_hook {
            continue;
        }

        let plugin_dir = state.plugins_dir.join(&plugin.id);
        let manifest_path = plugin_dir.join("plugin.toml");
        if !manifest_path.exists() {
            continue;
        }

        let manifest_content =
            std::fs::read_to_string(&manifest_path).map_err(|e| IpcErrorPayload {
                code: "PLUGIN_MANIFEST_ERROR".into(),
                message: format!("Failed to read plugin.toml for {}: {e}", plugin.id),
                recoverable: false,
            })?;

        let manifest =
            gitupload_core::plugin_manifest::PluginManifest::parse_toml(&manifest_content)
                .map_err(|e| IpcErrorPayload {
                    code: "PLUGIN_MANIFEST_INVALID".into(),
                    message: e,
                    recoverable: false,
                })?;

        state
            .plugin_runner
            .run_commit_msg_hook(&plugin_dir, &manifest, &commit_msg, &branch, &staged_files)
            .map_err(|e| IpcErrorPayload {
                code: "HOOK_REJECTED".into(),
                message: format!("[Plugin: {}] {}", plugin.name, e),
                recoverable: false,
            })?;
    }

    Ok(())
}
