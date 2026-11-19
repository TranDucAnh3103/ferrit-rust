//! Strict Enterprise Air-Gap network enforcement guard blocking external telemetry and cloud calls.

use crate::errors::{GitUploadError, GitUploadResult};
use std::sync::atomic::{AtomicBool, Ordering};
use url::Url;

static AIRGAP_ENFORCED: AtomicBool = AtomicBool::new(false);

/// Process-wide network boundary guard enforcing strict offline / intranet-only communication.
pub struct AirGapGuard;

impl AirGapGuard {
    /// Activates strict enterprise Air-Gap mode across outbound network clients.
    pub fn enable() {
        AIRGAP_ENFORCED.store(true, Ordering::SeqCst);
    }

    /// Deactivates Air-Gap mode (rejected at IPC layer when locked by GPO/enterprise policy).
    pub fn disable() {
        AIRGAP_ENFORCED.store(false, Ordering::SeqCst);
    }

    /// Returns `true` when strict Enterprise Air-Gap mode is active.
    pub fn is_enabled() -> bool {
        AIRGAP_ENFORCED.load(Ordering::SeqCst)
    }

    /// Determines whether `host` belongs to loopback or a local/corporate private intranet.
    pub fn is_local_or_intranet_host(host: &str) -> bool {
        let lower = host.trim().to_ascii_lowercase();
        if lower.is_empty() {
            return false;
        }
        if lower == "localhost"
            || lower == "127.0.0.1"
            || lower == "::1"
            || lower == "[::1]"
            || lower.ends_with(".local")
            || lower.ends_with(".internal")
            || lower.ends_with(".lan")
        {
            return true;
        }
        if let Ok(ip) = lower.parse::<std::net::Ipv4Addr>() {
            return ip.is_loopback() || ip.is_private() || ip.is_link_local();
        }
        false
    }

    /// Asserts that an outbound request targeting `url_or_host` is permitted under active Air-Gap settings.
    pub fn assert_network_allowed(url_or_host: &str) -> GitUploadResult<()> {
        if !Self::is_enabled() {
            return Ok(());
        }

        let host = if let Ok(parsed) = Url::parse(url_or_host) {
            parsed.host_str().unwrap_or("").to_string()
        } else {
            url_or_host
                .split(':')
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        };

        if Self::is_local_or_intranet_host(&host) {
            Ok(())
        } else {
            Err(GitUploadError::SafetyGuard(format!(
                "Outbound request to '{host}' blocked: Strict Enterprise Air-Gap Mode is enforced."
            )))
        }
    }
}
