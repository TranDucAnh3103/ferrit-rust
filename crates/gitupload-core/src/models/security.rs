//! DevSecOps vulnerability finding, severity, CWE/OWASP metadata, and patch proposal models.

use serde::{Deserialize, Serialize};

/// Severity classification aligned with CVSS / DevSecOps blocking thresholds.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum VulnerabilitySeverity {
    /// Immediate credential leak or remote code execution risk (blocks push).
    Critical,
    /// Serious exploitability or security boundary weakness (blocks push).
    High,
    /// Moderate risk issue requiring attention.
    Medium,
    /// Minor defense-in-depth hardening suggestion.
    Low,
    /// Informational audit note.
    Informational,
}

/// Automated single-file security patch proposal synthesized by Cloud AI.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SecurityPatchProposal {
    /// Human-readable summary of the fix.
    pub patch_description: String,
    /// Exact original vulnerable snippet to replace.
    pub original_code: String,
    /// Safe remediated replacement snippet.
    pub replacement_code: String,
    /// Optional unified diff preview.
    pub diff_unified: String,
}

/// Single security finding detected by static heuristics or Cloud AI semantic review.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct VulnerabilityFinding {
    /// Unique finding identifier (UUID v4).
    pub id: String,
    /// Short finding title (e.g. `"SQL Injection Vulnerability"`).
    pub title: String,
    /// Detailed explanation of the security risk.
    pub description: String,
    /// Finding severity classification.
    pub severity: VulnerabilitySeverity,
    /// Optional CWE identifier (e.g. `"CWE-89"`).
    pub cwe_id: Option<String>,
    /// Optional OWASP category reference (e.g. `"A03:2021-Injection"`).
    pub owasp_category: Option<String>,
    /// Relative repository path of the inspected file.
    pub file_path: String,
    /// 1-based start line number.
    pub line_start: usize,
    /// 1-based end line number.
    pub line_end: usize,
    /// Matched vulnerable snippet (with any raw secret masked before logging/UI).
    pub vulnerable_snippet: String,
    /// Optional 1-click patch proposal.
    pub suggested_fix: Option<SecurityPatchProposal>,
}

/// Aggregated DevSecOps pre-push security audit report.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct SecurityAuditReport {
    /// Unique scan identifier.
    pub scan_id: String,
    /// Target repository path.
    pub repo_path: String,
    /// Total count of all findings.
    pub total_findings: usize,
    /// Count of `Critical` findings.
    pub critical_count: usize,
    /// Count of `High` findings.
    pub high_count: usize,
    /// Count of `Medium` findings.
    pub medium_count: usize,
    /// Count of `Low` + `Informational` findings.
    pub low_count: usize,
    /// Full list of security findings.
    pub findings: Vec<VulnerabilityFinding>,
    /// Elapsed scan time in milliseconds.
    pub scan_duration_ms: u64,
    /// `false` if any `Critical` or `High` finding is present (pre-push blocking gate).
    pub passed: bool,
}
