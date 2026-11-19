//! Pure Cloud AI IPC commands (`ai_*`) with SSE chunk emission.

use crate::state::AppState;
use gitupload_ai::{AiGenerators, ChatMessage, CloudAiProvider};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_storage::SettingsRepository;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct AiStreamChunkPayload {
    stream_id: String,
    chunk: String,
    done: bool,
}

/// Generates a Conventional Commit message from `diff` via the configured Cloud AI provider.
#[tauri::command]
pub async fn ai_generate_commit(
    state: State<'_, AppState>,
    diff: String,
) -> Result<String, IpcErrorPayload> {
    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;
    let provider = CloudAiProvider::from_name(&settings.cloud_ai_provider);
    let key = state
        .get_ai_key(&settings.cloud_ai_provider)
        .map_err(IpcErrorPayload::from)?
        .or_else(|| state.get_ai_key("default").ok().flatten());

    if let Some(api_key) = key {
        state
            .ai
            .generate_commit_msg_with_key(&provider, &api_key, &settings.cloud_ai_model, &diff)
            .await
            .map_err(IpcErrorPayload::from)
    } else {
        state
            .ai
            .generate_commit_msg(&diff)
            .await
            .map_err(IpcErrorPayload::from)
    }
}

/// Generates semantic release changelog notes from commit summaries via Cloud AI.
#[tauri::command]
pub async fn ai_generate_changelog(
    state: State<'_, AppState>,
    tag_name: String,
    commits: Vec<String>,
) -> Result<String, IpcErrorPayload> {
    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;
    let provider = CloudAiProvider::from_name(&settings.cloud_ai_provider);
    let key = state
        .get_ai_key(&settings.cloud_ai_provider)
        .map_err(IpcErrorPayload::from)?
        .or_else(|| state.get_ai_key("default").ok().flatten())
        .ok_or_else(|| {
            GitUploadError::CloudAi("Missing Cloud AI API key in Windows Credential Manager".into())
        })?;

    AiGenerators::generate_release_changelog(
        state.ai.service(),
        &provider,
        &key,
        &settings.cloud_ai_model,
        &tag_name,
        &commits,
    )
    .await
    .map_err(|e| IpcErrorPayload::from(GitUploadError::from(e)))
}

/// Sends a message to Stephen Cloud AI Copilot, checking zero-token rules before streaming SSE chunks.
#[tauri::command]
pub async fn ai_chat_send(
    app: AppHandle,
    state: State<'_, AppState>,
    stream_id: String,
    history: Vec<ChatMessage>,
    user_input: String,
) -> Result<String, IpcErrorPayload> {
    let settings = SettingsRepository::new(&state.storage)
        .load_app_settings()
        .map_err(IpcErrorPayload::from)?;
    let provider = CloudAiProvider::from_name(&settings.cloud_ai_provider);
    let key = state
        .get_ai_key(&settings.cloud_ai_provider)
        .map_err(IpcErrorPayload::from)?
        .or_else(|| state.get_ai_key("default").ok().flatten());

    let app_clone = app.clone();
    let sid = stream_id.clone();
    let response = state
        .ai
        .chat(
            &provider,
            key.as_ref(),
            &settings.cloud_ai_model,
            &history,
            &user_input,
            move |chunk| {
                let _ = app_clone.emit(
                    "ai-stream-chunk",
                    AiStreamChunkPayload {
                        stream_id: sid.clone(),
                        chunk: chunk.to_string(),
                        done: false,
                    },
                );
            },
        )
        .await
        .map_err(IpcErrorPayload::from)?;

    let _ = app.emit(
        "ai-stream-chunk",
        AiStreamChunkPayload {
            stream_id,
            chunk: String::new(),
            done: true,
        },
    );

    Ok(response)
}
