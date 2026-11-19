//! 3-way merge conflict inspection, hunk preview, Cloud AI reconciliation, and commit finalization (`merge_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_ai::{CloudAiProvider, MergeAssistant};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{ConflictHunk, FileResolutionRequest, MergeConflictSession};
use gitupload_git::{CommitBuilder, ConflictResolver, MergeCalculator};
use gitupload_storage::SettingsRepository;
use tauri::State;

/// Detects 3-way merge conflicts between `HEAD` and `incoming_ref` without mutating files.
#[tauri::command]
pub async fn merge_detect_conflicts(
    repo_path: String,
    incoming_ref: String,
) -> Result<MergeConflictSession, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    tokio::task::spawn_blocking(move || {
        MergeCalculator::detect_conflicts(&safe_path, &incoming_ref)
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(IpcErrorPayload::from)
}

/// Applies resolution choices to raw conflicted text and returns clean resolved preview text.
#[tauri::command]
pub async fn merge_preview_resolution(
    raw_conflicted_text: String,
    hunks: Vec<ConflictHunk>,
    request: FileResolutionRequest,
) -> Result<String, IpcErrorPayload> {
    ConflictResolver::apply_resolutions(&raw_conflicted_text, &hunks, &request)
        .map_err(IpcErrorPayload::from)
}

/// Uses Cloud AI (Gemini / OpenAI / Claude / Groq / OpenRouter) to propose a clean merge for a single hunk.
#[tauri::command]
pub async fn merge_ai_auto_resolve(
    state: State<'_, AppState>,
    hunk: ConflictHunk,
) -> Result<String, IpcErrorPayload> {
    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;
    let provider = CloudAiProvider::from_name(&settings.cloud_ai_provider);
    let api_key = state
        .get_ai_key(&settings.cloud_ai_provider)
        .map_err(IpcErrorPayload::from)?
        .or_else(|| state.get_ai_key("default").ok().flatten())
        .ok_or_else(|| {
            IpcErrorPayload::from(GitUploadError::Vault(
                "No Cloud AI API key configured in Settings. Configure Gemini/OpenAI/Claude key first.".into(),
            ))
        })?;

    MergeAssistant::auto_resolve_conflict(
        &state.ai_client,
        &provider,
        &api_key,
        &settings.cloud_ai_model,
        &hunk,
    )
    .await
    .map_err(|e| IpcErrorPayload::from(GitUploadError::CloudAi(e.to_string())))
}

/// Writes resolved buffers to disk, stages zero-conflict paths, and records an atomic 2-parent merge commit.
#[tauri::command]
pub async fn merge_finalize_commit(
    repo_path: String,
    incoming_ref: String,
    resolutions: Vec<FileResolutionRequest>,
    commit_message: Option<String>,
) -> Result<String, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    tokio::task::spawn_blocking(move || {
        CommitBuilder::finalize_merge(
            &safe_path,
            &incoming_ref,
            &resolutions,
            commit_message.as_deref(),
        )
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(IpcErrorPayload::from)
}
