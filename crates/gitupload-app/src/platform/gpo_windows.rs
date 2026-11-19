//! Enterprise governance enforcement merging Windows Group Policy (`HKLM\Software\Policies\GitUpload`)
//! and cross-platform `/etc/gitupload/policy.json` (`%ProgramData%\GitUpload\policy.json`).

use gitupload_core::{AirGapGuard, EnterprisePolicyFile};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Resolved effective enterprise governance policy applied across UI and outbound clients.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectiveEnterprisePolicy {
    /// Governing IT organization label.
    pub organization_name: String,
    /// True when Windows GPO registry enforces Cloud AI shutdown.
    pub gpo_locks_cloud_ai: bool,
    /// True when GPO or policy.json locks Air-Gap mode on permanently.
    pub gpo_locks_airgap: bool,
    /// Active Air-Gap state (`true` disables public internet endpoints).
    pub airgap_enforced: bool,
    /// Whether Cloud AI is disabled by policy.
    pub cloud_ai_disabled: bool,
    /// Telemetry is always disabled in enterprise hardened mode.
    pub telemetry_disabled: bool,
    /// Optional corporate proxy endpoint URL enforced by IT.
    pub corporate_proxy_url: Option<String>,
    /// Permitted Git remote patterns/domains.
    pub allowed_remotes: Vec<String>,
    /// True when commit cryptographic signing is mandated.
    pub enforced_commit_sign: bool,
}

/// Evaluates Windows GPO registry (`HKLM\Software\Policies\GitUpload`) and system policy files.
pub struct EnterprisePolicyManager;

impl EnterprisePolicyManager {
    /// Loads effective enterprise policies from GPO registry + optional `policy.json` file path.
    pub fn load_effective_policy(policy_json_override: Option<&Path>) -> EffectiveEnterprisePolicy {
        let mut effective = EffectiveEnterprisePolicy {
            telemetry_disabled: true,
            ..Default::default()
        };

        let policy_path = policy_json_override
            .map(Path::to_path_buf)
            .unwrap_or_else(EnterprisePolicyFile::default_system_path);

        if let Ok(Some(file_policy)) = EnterprisePolicyFile::load_from_path(&policy_path) {
            let ent = file_policy.enterprise;
            effective.organization_name = ent.organization_name;
            effective.cloud_ai_disabled = ent.disable_cloud_ai;
            effective.telemetry_disabled = ent.disable_telemetry;
            effective.allowed_remotes = ent.allowed_remotes;
            effective.enforced_commit_sign = ent.enforced_commit_sign;
            if let Some(proxy) = ent.corporate_proxy_url.filter(|p| !p.trim().is_empty()) {
                effective.corporate_proxy_url = Some(proxy);
            }
            if ent.strict_airgap {
                effective.gpo_locks_airgap = true;
                effective.airgap_enforced = true;
            }
            if ent.disable_cloud_ai {
                effective.gpo_locks_cloud_ai = true;
            }
        }

        #[cfg(windows)]
        {
            if read_hklm_dword("DisableCloudAI") == Some(1) {
                effective.gpo_locks_cloud_ai = true;
                effective.cloud_ai_disabled = true;
            }
            if read_hklm_dword("ForceAirGapMode") == Some(1) {
                effective.gpo_locks_airgap = true;
                effective.airgap_enforced = true;
            }
            if let Some(proxy_reg) = read_hklm_string("CorporateProxyUrl") {
                if !proxy_reg.trim().is_empty() {
                    effective.corporate_proxy_url = Some(proxy_reg);
                }
            }
        }

        if effective.airgap_enforced {
            AirGapGuard::enable();
        }

        effective
    }

    /// Checks directly whether Windows GPO Registry enforces `DisableCloudAI = 1`.
    pub fn is_cloud_ai_disabled_by_gpo() -> bool {
        #[cfg(windows)]
        {
            read_hklm_dword("DisableCloudAI") == Some(1)
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

#[cfg(windows)]
fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn read_hklm_dword(value_name: &str) -> Option<u32> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD,
    };

    let subkey = to_wide_null(r"Software\Policies\GitUpload");
    let val_wide = to_wide_null(value_name);
    let mut hkey: HKEY = std::ptr::null_mut();

    // SAFETY: Opening HKLM registry subkey read-only with stack-allocated null handle.
    let open_res =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) };
    if open_res != 0 || hkey.is_null() {
        return None;
    }

    let mut reg_type: u32 = 0;
    let mut data: u32 = 0;
    let mut data_len: u32 = std::mem::size_of::<u32>() as u32;

    // SAFETY: Reading 4-byte DWORD from valid open HKEY before closing it immediately.
    let query_res = unsafe {
        RegQueryValueExW(
            hkey,
            val_wide.as_ptr(),
            std::ptr::null(),
            &mut reg_type,
            (&mut data as *mut u32).cast::<u8>(),
            &mut data_len,
        )
    };
    // SAFETY: Closing valid HKEY opened above.
    unsafe { RegCloseKey(hkey) };

    if query_res == 0 && reg_type == REG_DWORD {
        Some(data)
    } else {
        None
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn read_hklm_string(value_name: &str) -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ,
    };

    let subkey = to_wide_null(r"Software\Policies\GitUpload");
    let val_wide = to_wide_null(value_name);
    let mut hkey: HKEY = std::ptr::null_mut();

    // SAFETY: Opening HKLM registry subkey read-only.
    let open_res =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) };
    if open_res != 0 || hkey.is_null() {
        return None;
    }

    let mut reg_type: u32 = 0;
    let mut buf = [0u16; 512];
    let mut byte_len: u32 = (buf.len() * 2) as u32;

    // SAFETY: Reading UTF-16 string into bounded 1024-byte stack buffer.
    let query_res = unsafe {
        RegQueryValueExW(
            hkey,
            val_wide.as_ptr(),
            std::ptr::null(),
            &mut reg_type,
            buf.as_mut_ptr().cast::<u8>(),
            &mut byte_len,
        )
    };
    // SAFETY: Closing valid HKEY opened above.
    unsafe { RegCloseKey(hkey) };

    if query_res == 0 && reg_type == REG_SZ && byte_len >= 2 {
        let u16_len = (byte_len as usize / 2).min(buf.len());
        let trimmed = buf[..u16_len]
            .iter()
            .take_while(|&&c| c != 0)
            .copied()
            .collect::<Vec<u16>>();
        String::from_utf16(&trimmed).ok()
    } else {
        None
    }
}
