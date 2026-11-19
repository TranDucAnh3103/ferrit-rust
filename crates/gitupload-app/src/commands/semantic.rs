//! Semantic Git commit vector indexing & sub-50ms cosine similarity search IPC commands (`ai_index_*`, `ai_semantic_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use gitupload_ai::{CloudAiProvider, EmbeddingsClient};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::{SearchHit, VectorSearchEngine};
use gitupload_git::CommitIndexer;
use gitupload_storage::{EmbeddingStore, SettingsRepository};
use std::collections::HashSet;
use tauri::State;

/// Incrementally embeds unindexed commits via Cloud AI and caches BLOB vectors in SQLite.
#[tauri::command]
pub async fn ai_index_commits(
    state: State<'_, AppState>,
    repo_path: String,
    max_commits: Option<usize>,
) -> Result<usize, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();

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
                "Configure a Gemini or OpenAI API key in Settings to generate commit embeddings."
                    .into(),
            ))
        })?;

    let store = EmbeddingStore::new(&state.storage);
    let existing = store
        .load_repo_embeddings(&normalized)
        .map_err(IpcErrorPayload::from)?;
    let indexed_set: HashSet<String> = existing.into_iter().map(|(oid, _, _)| oid).collect();

    let limit = max_commits.unwrap_or(200);
    let path_clone = safe_path.clone();
    let pending = tokio::task::spawn_blocking(move || {
        CommitIndexer::collect_unindexed_commits(&path_clone, &indexed_set, limit)
    })
    .await
    .map_err(|e| GitUploadError::Internal(e.to_string()))?
    .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    let mut newly_indexed = 0usize;
    for (oid, summary) in pending {
        if let Ok(vec) =
            EmbeddingsClient::generate_embedding(&state.ai_client, &provider, &api_key, &summary)
                .await
        {
            if store
                .save_embedding(&normalized, &oid, &summary, &vec)
                .is_ok()
            {
                newly_indexed += 1;
            }
        }
    }

    Ok(newly_indexed)
}

/// Performs natural language semantic vector ranking over cached repository commit embeddings.
#[tauri::command]
pub async fn ai_semantic_search(
    state: State<'_, AppState>,
    repo_path: String,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<SearchHit>, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let normalized = safe_path.to_string_lossy().to_string();

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
                "Configure a Gemini or OpenAI API key in Settings to run semantic commit queries."
                    .into(),
            ))
        })?;

    let query_vec =
        EmbeddingsClient::generate_embedding(&state.ai_client, &provider, &api_key, &query)
            .await
            .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))?;

    let cached = EmbeddingStore::new(&state.storage)
        .load_repo_embeddings(&normalized)
        .map_err(IpcErrorPayload::from)?;

    Ok(VectorSearchEngine::rank_matches(
        &query_vec,
        &cached,
        top_k.unwrap_or(20),
        0.30,
    ))
}
