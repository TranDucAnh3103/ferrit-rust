#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_code)]

use gitupload_app::commands;
use gitupload_app::state::AppState;
use gitupload_storage::{LegacyImporter, StoragePool};
use tauri::Manager;
use tracing::{info, warn};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const MUTEX_NAME: &[u8] = b"Local\\FerritSingleInstanceMutex\0";

#[allow(unsafe_code)]
fn ensure_single_instance() -> Result<usize, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexA;

        // SAFETY: Invoking Win32 CreateMutexA with null security attributes and static null-terminated ASCII name.
        let handle = unsafe { CreateMutexA(std::ptr::null(), 0, MUTEX_NAME.as_ptr()) };
        if handle.is_null() {
            return Err("Failed to create Win32 single-instance mutex".into());
        }
        // SAFETY: Querying GetLastError immediately after CreateMutexA on the same thread.
        let last_err = unsafe { GetLastError() };
        if last_err == ERROR_ALREADY_EXISTS {
            return Err("FERRIT Desktop is already running on this machine.".into());
        }
        Ok(handle as usize)
    }
    #[cfg(not(windows))]
    {
        Ok(0)
    }
}

#[allow(unsafe_code)]
fn show_fatal_dialog(msg: &str, location: &str) {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let text: Vec<u16> = format!("FERRIT Desktop Error:\n\n{msg}\n\nLocation: {location}")
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let caption: Vec<u16> = "FERRIT Desktop"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        unsafe {
            MessageBoxW(
                0 as _,
                text.as_ptr(),
                caption.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(not(windows))]
    {
        eprintln!("FERRIT Fatal Error at {location}: {msg}");
    }
}

fn init_tracing() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "ferrit", "FerritDesktop")
        .or_else(|| directories::ProjectDirs::from("com", "gitupload", "GitUploadDesktop"))
    {
        let log_dir = proj_dirs.data_local_dir().join("logs");
        if std::fs::create_dir_all(&log_dir).is_ok() {
            let file_appender = tracing_appender::rolling::daily(log_dir, "ferrit.log");
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let _ = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(non_blocking)
                .try_init();
            return Some(guard);
        }
    }
    let _ = tracing_subscriber::fmt().try_init();
    None
}

#[cfg(target_os = "linux")]
fn configure_linux_display_backend() {
    if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("GDK_BACKEND").is_err() {
        std::env::set_var("GDK_BACKEND", "wayland,x11");
    }
    if std::env::var("WEBKIT_FORCE_COMPOSITING_MODE").is_err() {
        std::env::set_var("WEBKIT_FORCE_COMPOSITING_MODE", "1");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    configure_linux_display_backend();

    let _log_guard = init_tracing();

    std::panic::set_hook(Box::new(|info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".into());
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "An unexpected runtime error occurred in FERRIT Desktop.",
            },
        };
        tracing::error!("FATAL PANIC at {location}: {msg}");
        show_fatal_dialog(msg, &location);
    }));

    let _mutex_handle = match ensure_single_instance() {
        Ok(handle) => Some(handle),
        Err(err) => {
            warn!("{err}");
            None
        }
    };

    let _policy = gitupload_app::platform::EnterprisePolicyManager::load_effective_policy(None);
    info!("Starting FERRIT Desktop v2.6.0-enterprise...");

    let storage = match StoragePool::init_default() {
        Ok(pool) => pool,
        Err(err) => {
            warn!("Failed to initialize SQLite storage pool: {err}");
            std::process::exit(1);
        }
    };

    let state = AppState::new(storage);

    if let Err(err) = LegacyImporter::migrate_if_needed(&state.storage, &state.keyring) {
        warn!("Legacy migration skipped or failed: {err}");
    }

    let app_result = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::auth_save_token,
            commands::auth_save_ai_key,
            commands::auth_list_accounts,
            commands::auth_sync_from_gh,
            commands::git_inspect_repo,
            commands::git_get_diff,
            commands::git_start_push,
            commands::git_repush_repo,
            commands::git_start_clone,
            commands::git_cancel_job,
            commands::git_generate_gitignore,
            commands::graph_get_commits,
            commands::graph_cherry_pick,
            commands::graph_revert,
            commands::graph_reset,
            commands::graph_checkout,
            commands::graph_create_branch,
            commands::ghcli_check_status,
            commands::ghcli_install_silent,
            commands::org_list,
            commands::repo_search,
            commands::repo_delete,
            commands::repo_contents,
            commands::repo_readme,
            commands::repo_file_get,
            commands::repo_file_update,
            commands::repo_get_profile,
            commands::repo_token_report,
            commands::repo_branch_protection,
            commands::pr_list,
            commands::pr_create,
            commands::pr_merge,
            commands::actions_list_runs,
            commands::actions_rerun_failed,
            commands::actions_cancel_run,
            commands::issues_list,
            commands::issues_create,
            commands::releases_create,
            commands::hooks_install,
            commands::hooks_validate_commit,
            commands::ai_generate_commit,
            commands::ai_generate_changelog,
            commands::ai_chat_send,
            commands::system_detect_ides,
            commands::system_open_in_ide,
            commands::system_open_browser,
            commands::system_open_in_explorer,
            commands::system_get_settings,
            commands::system_save_settings,
            commands::system_get_history,
            commands::system_migrate_legacy,
            commands::system_get_guide,
            commands::system_watch_directory,
            commands::system_stop_watcher,
            commands::mirror_list_remotes,
            commands::mirror_upsert_remote,
            commands::mirror_delete_remote,
            commands::mirror_probe_remotes,
            commands::mirror_push_all,
            commands::merge_detect_conflicts,
            commands::merge_preview_resolution,
            commands::merge_ai_auto_resolve,
            commands::merge_finalize_commit,
            commands::ssh_list_keys,
            commands::ssh_generate_key,
            commands::ssh_delete_key,
            commands::ssh_agent_status,
            commands::ssh_agent_start,
            commands::ssh_agent_stop,
            commands::ssh_upload_key_to_github,
            commands::analytics_scan_repo,
            commands::analytics_get_heatmap,
            commands::analytics_get_velocity,
            commands::analytics_list_assignments,
            commands::analytics_upsert_assignment,
            commands::analytics_delete_assignment,
            commands::analytics_evaluate_compliance,
            commands::analytics_get_leaderboard,
            commands::security_run_audit,
            commands::security_apply_patch,
            commands::ai_index_commits,
            commands::ai_semantic_search,
            commands::git_get_stale_branches,
            commands::git_prune_branches,
            commands::sync_get_power_state,
            commands::sync_get_config,
            commands::sync_save_config,
            commands::sync_run_now,
            commands::sync_create_bundle,
            commands::updater_get_channel,
            commands::updater_set_channel,
            commands::updater_check_for_updates,
            commands::enterprise_get_policy,
            commands::enterprise_set_airgap,
            commands::enterprise_get_proxy,
            commands::enterprise_save_proxy,
            commands::enterprise_validate_remote_url,
            commands::webhook_list_endpoints,
            commands::webhook_upsert_endpoint,
            commands::webhook_delete_endpoint,
            commands::webhook_test_ping,
            commands::webhook_list_deliveries,
            commands::plugin_list_installed,
            commands::plugin_toggle_active,
            commands::plugin_uninstall,
            commands::plugin_execute_commit_hook,
            commands::marketplace_fetch_catalog,
            commands::marketplace_install_plugin,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = app_result {
        warn!("Error while running FERRIT Tauri v2 application: {err}");
        std::process::exit(1);
    }
}
