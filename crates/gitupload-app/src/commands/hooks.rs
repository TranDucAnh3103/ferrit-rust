//! Native pure-Rust Git hooks commands (`hooks_*`).

use crate::commands::validate_safe_path;
use gitupload_core::errors::IpcErrorPayload;
use gitupload_hooks::{HookInstallStatus, HookInstaller, HookRunner, HooksConfig};

/// Installs `.gitupload/hooks.toml` and pure-Rust hook shims into `<path>/.git/hooks`.
#[tauri::command]
pub async fn hooks_install(path: String) -> Result<HookInstallStatus, IpcErrorPayload> {
    let safe_path = validate_safe_path(&path)?;
    let cfg = HooksConfig::load_or_default(&safe_path).map_err(IpcErrorPayload::from)?;
    HookInstaller::install_into_repo(&safe_path, &cfg).map_err(IpcErrorPayload::from)
}

/// Validates a commit message against Conventional Commits 1.0.0 rules.
#[tauri::command]
pub async fn hooks_validate_commit(
    message: String,
    max_len: Option<usize>,
) -> Result<(), IpcErrorPayload> {
    HookRunner::validate_commit_msg(&message, max_len.unwrap_or(72)).map_err(IpcErrorPayload::from)
}
