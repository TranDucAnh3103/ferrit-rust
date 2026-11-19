#![forbid(unsafe_code)]
//! Enterprise governance, Air-Gap toggle, Windows GPO / policy.json readout, and corporate proxy IPC (`enterprise_*`).

use crate::platform::{EffectiveEnterprisePolicy, EnterprisePolicyManager};
use crate::state::AppState;
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::{AirGapGuard, EnterprisePolicyFile};
use gitupload_github::CorporateProxyConfig;
use gitupload_security::PlatformVault;
use gitupload_storage::HooksConfigRepository;
use serde::Serialize;
use tauri::State;

const AIRGAP_SETTING_KEY: &str = "enterprise_airgap_enabled";
const PROXY_SETTING_KEY: &str = "enterprise_corporate_proxy";

/// Cross-platform OS runtime & vault diagnostic metadata exposed to Enterprise view.
#[derive(Debug, Clone, Serialize)]
pub struct PlatformCapabilitiesInfo {
    /// Target OS name (`windows`, `macos`, `linux`).
    pub os_name: String,
    /// Target CPU architecture (`x86_64`, `aarch64`).
    pub arch: String,
    /// Active OS credential backend description.
    pub vault_backend: String,
    /// Effective enterprise governance policy snapshot.
    pub policy: EffectiveEnterprisePolicy,
}

/// Returns active platform capabilities alongside merged GPO + `policy.json` + local Air-Gap settings.
#[tauri::command]
pub async fn enterprise_get_policy(
    state: State<'_, AppState>,
) -> Result<PlatformCapabilitiesInfo, IpcErrorPayload> {
    let mut policy = EnterprisePolicyManager::load_effective_policy(None);
    let stored_airgap = HooksConfigRepository::new(&state.storage)
        .get(AIRGAP_SETTING_KEY)
        .map_err(IpcErrorPayload::from)?
        .is_some_and(|v| v == "true");

    if policy.gpo_locks_airgap || stored_airgap {
        policy.airgap_enforced = true;
        AirGapGuard::enable();
    } else {
        AirGapGuard::disable();
    }

    Ok(PlatformCapabilitiesInfo {
        os_name: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        vault_backend: PlatformVault::active_backend_name().to_string(),
        policy,
    })
}

/// Updates strict Air-Gap isolation mode (rejects disabling if locked by IT GPO or `policy.json`).
#[tauri::command]
pub async fn enterprise_set_airgap(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<EffectiveEnterprisePolicy, IpcErrorPayload> {
    let mut policy = EnterprisePolicyManager::load_effective_policy(None);
    if policy.gpo_locks_airgap && !enabled {
        return Err(IpcErrorPayload::from(GitUploadError::SafetyGuard(
            "Cannot disable Air-Gap Mode: enforced by Windows GPO or system policy.json.".into(),
        )));
    }

    HooksConfigRepository::new(&state.storage)
        .set(AIRGAP_SETTING_KEY, if enabled { "true" } else { "false" })
        .map_err(IpcErrorPayload::from)?;

    if enabled {
        AirGapGuard::enable();
    } else {
        AirGapGuard::disable();
    }
    policy.airgap_enforced = enabled;
    Ok(policy)
}

/// Loads saved corporate proxy & custom Root CA certificate configuration.
#[tauri::command]
pub async fn enterprise_get_proxy(
    state: State<'_, AppState>,
) -> Result<CorporateProxyConfig, IpcErrorPayload> {
    let raw = HooksConfigRepository::new(&state.storage)
        .get(PROXY_SETTING_KEY)
        .map_err(IpcErrorPayload::from)?;
    Ok(raw
        .and_then(|json| serde_json::from_str::<CorporateProxyConfig>(&json).ok())
        .unwrap_or_default())
}

/// Validates corporate proxy / CA certificate PEM construction and persists settings cleanly.
#[tauri::command]
pub async fn enterprise_save_proxy(
    state: State<'_, AppState>,
    config: CorporateProxyConfig,
) -> Result<(), IpcErrorPayload> {
    // Validate HTTP client + PEM CA certificate syntax upfront before persisting
    config
        .build_http_client()
        .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    let json = serde_json::to_string(&config)
        .map_err(|e| IpcErrorPayload::from(GitUploadError::Serialization(e)))?;
    HooksConfigRepository::new(&state.storage)
        .set(PROXY_SETTING_KEY, &json)
        .map_err(IpcErrorPayload::from)
}

/// Evaluates whether a Git remote URL complies with `policy.json` & `AirGapGuard`.
#[tauri::command]
pub async fn enterprise_validate_remote_url(url: String) -> Result<bool, IpcErrorPayload> {
    if AirGapGuard::assert_network_allowed(&url).is_err() {
        return Ok(false);
    }
    let sys_path = EnterprisePolicyFile::default_system_path();
    if let Ok(Some(pol)) = EnterprisePolicyFile::load_from_path(&sys_path) {
        return Ok(pol.is_remote_allowed(&url));
    }
    Ok(true)
}
