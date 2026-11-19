//! Cloud AI DevSecOps diff evaluator detecting OWASP Top 10 & CWE vulnerabilities.

use crate::client::{ChatMessage, CloudAiProvider, CloudAiService};
use crate::merge_assistant::strip_markdown_fences;
use gitupload_core::errors::{AiError, AiResult};
use gitupload_core::models::{SecurityPatchProposal, VulnerabilityFinding, VulnerabilitySeverity};
use secrecy::SecretString;

/// Evaluates staged/working diffs semantically via Cloud AI for OWASP Top 10 and CWE defects.
pub struct CloudAiSecurityEvaluator;

impl CloudAiSecurityEvaluator {
    /// Analyzes `diff_text` for `file_path` via Cloud AI and returns structured [`VulnerabilityFinding`]s.
    pub async fn evaluate_diff(
        ai: &CloudAiService,
        provider: &CloudAiProvider,
        api_key: &SecretString,
        model: &str,
        file_path: &str,
        diff_text: &str,
    ) -> AiResult<Vec<VulnerabilityFinding>> {
        if diff_text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let system_prompt = r#"You are a principal application security engineer. Analyze the provided Git diff for security vulnerabilities (OWASP Top 10, CWE).
Respond ONLY with a JSON array conforming to this schema:
[
  {
    "title": "string",
    "description": "string",
    "severity": "Critical" | "High" | "Medium" | "Low",
    "cwe_id": "string",
    "owasp_category": "string",
    "line_start": number,
    "line_end": number,
    "vulnerable_snippet": "string",
    "patch_description": "string",
    "replacement_code": "string"
  }
]
If no vulnerabilities exist, return an empty array []. Do not output markdown code fences."#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Analyze file '{file_path}' with diff:\n\n{diff_text}"),
            },
        ];

        let response = ai
            .stream_completion(provider, api_key, model, &messages, |_| {})
            .await?;

        Self::parse_findings_json(file_path, &response)
    }

    /// Parses raw Cloud AI JSON response text into structured [`VulnerabilityFinding`] objects.
    pub fn parse_findings_json(
        file_path: &str,
        raw_response: &str,
    ) -> AiResult<Vec<VulnerabilityFinding>> {
        let trimmed = strip_markdown_fences(raw_response);
        if trimmed.is_empty() || trimmed == "[]" {
            return Ok(Vec::new());
        }

        let raw_findings: Vec<serde_json::Value> = serde_json::from_str(&trimmed).map_err(|e| {
            AiError::InvalidResponse(format!(
                "Failed to parse security audit JSON: {e} (Output: {trimmed})"
            ))
        })?;

        let findings = raw_findings
            .into_iter()
            .map(|f| {
                let severity = match f["severity"].as_str().unwrap_or("Medium") {
                    "Critical" => VulnerabilitySeverity::Critical,
                    "High" => VulnerabilitySeverity::High,
                    "Low" => VulnerabilitySeverity::Low,
                    "Informational" => VulnerabilitySeverity::Informational,
                    _ => VulnerabilitySeverity::Medium,
                };

                let replacement = f["replacement_code"].as_str().map(String::from);
                let patch_desc = f["patch_description"]
                    .as_str()
                    .unwrap_or("Apply security remediation")
                    .to_string();
                let original = f["vulnerable_snippet"].as_str().unwrap_or("").to_string();

                let suggested_fix = replacement.map(|r| SecurityPatchProposal {
                    patch_description: patch_desc,
                    original_code: original.clone(),
                    replacement_code: r,
                    diff_unified: String::new(),
                });

                VulnerabilityFinding {
                    id: uuid::Uuid::new_v4().to_string(),
                    title: f["title"]
                        .as_str()
                        .unwrap_or("Security Finding")
                        .to_string(),
                    description: f["description"].as_str().unwrap_or("").to_string(),
                    severity,
                    cwe_id: f["cwe_id"].as_str().map(String::from),
                    owasp_category: f["owasp_category"].as_str().map(String::from),
                    file_path: file_path.to_string(),
                    line_start: f["line_start"].as_u64().unwrap_or(1) as usize,
                    line_end: f["line_end"].as_u64().unwrap_or(1) as usize,
                    vulnerable_snippet: original,
                    suggested_fix,
                }
            })
            .collect();

        Ok(findings)
    }
}
