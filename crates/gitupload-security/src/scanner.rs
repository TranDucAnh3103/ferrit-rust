#![forbid(unsafe_code)]
//! Fast sub-millisecond local static secret, keypair, and unencrypted env-file scanner.

use crate::sanitizer::sanitize_log_message;
use gitupload_core::models::{VulnerabilityFinding, VulnerabilitySeverity};
use regex::Regex;
use std::sync::LazyLock;

struct RuleSpec {
    title: &'static str,
    cwe: &'static str,
    owasp: &'static str,
    severity: VulnerabilitySeverity,
    regex: Option<Regex>,
}

static STATIC_RULES: LazyLock<Vec<RuleSpec>> = LazyLock::new(|| {
    vec![
        RuleSpec {
            title: "Hardcoded GitHub Personal Access Token",
            cwe: "CWE-798",
            owasp: "A07:2021-Identification and Authentication Failures",
            severity: VulnerabilitySeverity::Critical,
            regex: Regex::new(r"gh[pousr]_[A-Za-z0-9_]{30,255}").ok(),
        },
        RuleSpec {
            title: "GitHub Fine-Grained Personal Access Token",
            cwe: "CWE-798",
            owasp: "A07:2021-Identification and Authentication Failures",
            severity: VulnerabilitySeverity::Critical,
            regex: Regex::new(r"github_pat_[A-Za-z0-9_]{20,255}").ok(),
        },
        RuleSpec {
            title: "AWS Access Key ID",
            cwe: "CWE-798",
            owasp: "A07:2021-Identification and Authentication Failures",
            severity: VulnerabilitySeverity::Critical,
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").ok(),
        },
        RuleSpec {
            title: "Generic Private Cryptographic Key",
            cwe: "CWE-312",
            owasp: "A02:2021-Cryptographic Failures",
            severity: VulnerabilitySeverity::Critical,
            regex: Regex::new(r"-----BEGIN (RSA|EC|OPENSSH|PGP|DSA) PRIVATE KEY-----").ok(),
        },
        RuleSpec {
            title: "Hardcoded High-Entropy API Secret",
            cwe: "CWE-798",
            owasp: "A07:2021-Identification and Authentication Failures",
            severity: VulnerabilitySeverity::High,
            regex: Regex::new(
                r#"(?i)(api[_-]?key|secret[_-]?key|auth[_-]?token|client[_-]?secret)\s*[:=]\s*["'][A-Za-z0-9_\-]{16,120}["']"#,
            )
            .ok(),
        },
    ]
});

/// Sub-millisecond local regex/path heuristic security scanner operating prior to network transmission.
pub struct StaticSecretScanner;

impl StaticSecretScanner {
    /// Scans `file_path` and its UTF-8 `content` (diff lines or file text) for secrets and tracked `.env` files.
    pub fn scan_content(file_path: &str, content: &str) -> Vec<VulnerabilityFinding> {
        let mut findings = Vec::new();
        let norm_path = file_path.replace('\\', "/");
        let file_name = norm_path.rsplit('/').next().unwrap_or(&norm_path);

        if file_name == ".env" || file_name.starts_with(".env.") {
            findings.push(VulnerabilityFinding {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Unencrypted Environment File Tracked".to_string(),
                description: format!(
                    "File '{file_path}' appears to be an environment file containing sensitive runtime configuration."
                ),
                severity: VulnerabilitySeverity::Critical,
                cwe_id: Some("CWE-312".to_string()),
                owasp_category: Some("A05:2021-Security Misconfiguration".to_string()),
                file_path: file_path.to_string(),
                line_start: 1,
                line_end: 1,
                vulnerable_snippet: file_path.to_string(),
                suggested_fix: None,
            });
        }

        for (line_no, line) in content.lines().enumerate() {
            for rule in STATIC_RULES.iter() {
                let Some(ref regex) = rule.regex else {
                    continue;
                };
                if regex.is_match(line) {
                    findings.push(VulnerabilityFinding {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: rule.title.to_string(),
                        description: format!(
                            "Detected potential hardcoded secret or credential on line {}.",
                            line_no + 1
                        ),
                        severity: rule.severity.clone(),
                        cwe_id: Some(rule.cwe.to_string()),
                        owasp_category: Some(rule.owasp.to_string()),
                        file_path: file_path.to_string(),
                        line_start: line_no + 1,
                        line_end: line_no + 1,
                        vulnerable_snippet: sanitize_log_message(line.trim()),
                        suggested_fix: None,
                    });
                }
            }
        }

        findings
    }
}
