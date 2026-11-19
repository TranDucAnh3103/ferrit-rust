//! DevSecOps pre-push vulnerability auditing & 1-click AI patch IPC commands (`security_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_ai::{CloudAiProvider, CloudAiSecurityEvaluator};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{
    SecurityAuditReport, SecurityPatchProposal, VulnerabilityFinding, VulnerabilitySeverity,
};
use gitupload_git::AutomatedPatcher;
use gitupload_security::StaticSecretScanner;
use gitupload_storage::{EmbeddingStore, SettingsRepository};
use std::time::Instant;
use tauri::State;

/// Executes static heuristic secret scanning + optional Cloud AI semantic audit on dirty files.
#[tauri::command]
pub async fn security_run_audit(
    state: State<'_, AppState>,
    repo_path: String,
    use_cloud_ai: Option<bool>,
) -> Result<SecurityAuditReport, IpcErrorPayload> {
    let started = Instant::now();
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();

    let scan_path = safe_path.clone();
    let files =
        tokio::task::spawn_blocking(move || AutomatedPatcher::collect_dirty_files(&scan_path, 40))
            .await
            .map_err(|e| GitUploadError::Internal(e.to_string()))?
            .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    let mut findings: Vec<VulnerabilityFinding> = Vec::new();
    for (rel_path, content) in &files {
        findings.extend(StaticSecretScanner::scan_content(rel_path, content));
    }

    if use_cloud_ai.unwrap_or(true) && !files.is_empty() {
        let settings = SettingsRepository::new(&state.storage)
            .load_app_settings()
            .map_err(IpcErrorPayload::from)?;
        let provider = CloudAiProvider::from_name(&settings.cloud_ai_provider);
        if let Ok(Some(api_key)) = state
            .get_ai_key(&settings.cloud_ai_provider)
            .map(|opt| opt.or_else(|| state.get_ai_key("default").ok().flatten()))
        {
            for (rel_path, content) in files.iter().take(6) {
                if let Ok(ai_findings) = CloudAiSecurityEvaluator::evaluate_diff(
                    &state.ai_client,
                    &provider,
                    &api_key,
                    &settings.cloud_ai_model,
                    rel_path,
                    content,
                )
                .await
                {
                    findings.extend(ai_findings);
                }
            }
        }
    }

    let critical_count = findings
        .iter()
        .filter(|f| f.severity == VulnerabilitySeverity::Critical)
        .count();
    let high_count = findings
        .iter()
        .filter(|f| f.severity == VulnerabilitySeverity::High)
        .count();
    let medium_count = findings
        .iter()
        .filter(|f| f.severity == VulnerabilitySeverity::Medium)
        .count();
    let low_count = findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                VulnerabilitySeverity::Low | VulnerabilitySeverity::Informational
            )
        })
        .count();

    let report = SecurityAuditReport {
        scan_id: uuid::Uuid::new_v4().to_string(),
        repo_path: normalized,
        total_findings: findings.len(),
        critical_count,
        high_count,
        medium_count,
        low_count,
        findings,
        scan_duration_ms: started.elapsed().as_millis() as u64,
        passed: critical_count == 0 && high_count == 0,
    };

    let _ = EmbeddingStore::new(&state.storage).record_security_scan(&report);
    Ok(report)
}

/// Applies an AI-proposed 1-click security patch safely inside `repo_path`.
#[tauri::command]
pub async fn security_apply_patch(
    repo_path: String,
    file_path: String,
    proposal: SecurityPatchProposal,
) -> Result<(), IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    tokio::task::spawn_blocking(move || {
        AutomatedPatcher::apply_patch(&safe_path, &file_path, &proposal)
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}
