//! Real-time regex log sanitization preventing accidental leakage of PATs, bearer tokens, and API keys.

use regex::Regex;
use std::sync::OnceLock;

static RE_URL_TOKEN: OnceLock<Option<Regex>> = OnceLock::new();
static RE_GH_FINE_PAT: OnceLock<Option<Regex>> = OnceLock::new();
static RE_GH_TOKEN: OnceLock<Option<Regex>> = OnceLock::new();
static RE_BEARER: OnceLock<Option<Regex>> = OnceLock::new();
static RE_API_KEY: OnceLock<Option<Regex>> = OnceLock::new();
static RE_MASK_COMBINED: OnceLock<Option<Regex>> = OnceLock::new();

fn re_url_token() -> Option<&'static Regex> {
    RE_URL_TOKEN
        .get_or_init(|| Regex::new(r"https://[^@\s/]+@github\.com").ok())
        .as_ref()
}

fn re_gh_fine_pat() -> Option<&'static Regex> {
    RE_GH_FINE_PAT
        .get_or_init(|| Regex::new(r"github_pat_[A-Za-z0-9_]{20,255}").ok())
        .as_ref()
}

fn re_gh_token() -> Option<&'static Regex> {
    RE_GH_TOKEN
        .get_or_init(|| Regex::new(r"gh[pousr]_[A-Za-z0-9_]{20,255}").ok())
        .as_ref()
}

fn re_bearer() -> Option<&'static Regex> {
    RE_BEARER
        .get_or_init(|| Regex::new(r"Bearer\s+[A-Za-z0-9\-._~+/]+=*").ok())
        .as_ref()
}

fn re_api_key() -> Option<&'static Regex> {
    RE_API_KEY
        .get_or_init(|| {
            Regex::new(r"(sk-[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{20,}|AKIA[0-9A-Z]{16})").ok()
        })
        .as_ref()
}

fn re_mask_combined() -> Option<&'static Regex> {
    RE_MASK_COMBINED
        .get_or_init(|| {
            Regex::new(r"(gh[pousr]_[A-Za-z0-9_]{20,255}|github_pat_[A-Za-z0-9_]{20,255}|sk-[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{20,}|AKIA[0-9A-Z]{16})").ok()
        })
        .as_ref()
}

/// Masks sensitive tokens in raw command/log lines (`https://***@github.com` and `***`).
pub fn mask_sensitive_tokens(raw: &str) -> String {
    let step1 = match re_url_token() {
        Some(re) => re.replace_all(raw, "https://***@github.com").into_owned(),
        None => raw.to_string(),
    };
    let step2 = match re_bearer() {
        Some(re) => re.replace_all(&step1, "Bearer ***").into_owned(),
        None => step1,
    };
    match re_mask_combined() {
        Some(re) => re.replace_all(&step2, "***").into_owned(),
        None => step2,
    }
}

/// Sanitizes log lines and IPC telemetry streams using explicit redaction tags per SEC-02 policy.
pub fn sanitize_log_message(raw: &str) -> String {
    let step1 = match re_url_token() {
        Some(re) => re.replace_all(raw, "https://***@github.com").into_owned(),
        None => raw.to_string(),
    };
    let step2 = match re_gh_fine_pat() {
        Some(re) => re.replace_all(&step1, "[REDACTED_PAT]").into_owned(),
        None => step1,
    };
    let step3 = match re_gh_token() {
        Some(re) => re.replace_all(&step2, "[REDACTED_TOKEN]").into_owned(),
        None => step2,
    };
    let step4 = match re_bearer() {
        Some(re) => re.replace_all(&step3, "Bearer [REDACTED]").into_owned(),
        None => step3,
    };
    match re_api_key() {
        Some(re) => re.replace_all(&step4, "[REDACTED_KEY]").into_owned(),
        None => step4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking() {
        let raw = "git remote add origin https://ghp_abc@github.com/org/repo";
        let masked = mask_sensitive_tokens(raw);
        assert_eq!(
            masked,
            "git remote add origin https://***@github.com/org/repo"
        );
        assert!(!masked.contains("ghp_abc"));

        let standalone = "token=ghp_1234567890abcdef1234567890abcdef12 key=sk-1234567890abcdefghij";
        let masked2 = mask_sensitive_tokens(standalone);
        assert!(!masked2.contains("ghp_1234"));
        assert!(!masked2.contains("sk-1234"));
        assert!(masked2.contains("***"));
    }

    #[test]
    fn test_sanitize_log_message_sec02() {
        let fine_pat = format!("github_pat_{}", "a".repeat(82));
        let log = format!(
            "Authorization: Bearer secret123 | PAT: ghp_1234567890abcdef1234567890abcdef12 | Fine: {} | URL: https://ghp_xyz@github.com/u/r",
            fine_pat
        );
        let sanitized = sanitize_log_message(&log);
        assert!(sanitized.contains("Bearer [REDACTED]"));
        assert!(sanitized.contains("[REDACTED_TOKEN]"));
        assert!(sanitized.contains("[REDACTED_PAT]"));
        assert!(sanitized.contains("https://***@github.com/u/r"));
        assert!(!sanitized.contains("secret123"));
        assert!(!sanitized.contains("ghp_"));
        assert!(!sanitized.contains("github_pat_"));
    }
}
