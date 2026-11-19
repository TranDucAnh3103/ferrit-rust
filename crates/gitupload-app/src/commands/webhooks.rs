use crate::state::AppState;
use gitupload_core::errors::IpcErrorPayload;
use gitupload_core::webhooks::{WebhookDeliveryRecord, WebhookEndpoint};
use gitupload_storage::WebhooksRepository;
use secrecy::SecretString;
use tauri::State;

#[tauri::command]
pub async fn webhook_list_endpoints(
    state: State<'_, AppState>,
) -> Result<Vec<WebhookEndpoint>, IpcErrorPayload> {
    let repo = WebhooksRepository::new(state.storage.clone());
    repo.list_endpoints().map_err(IpcErrorPayload::from)
}

#[tauri::command]
pub async fn webhook_upsert_endpoint(
    state: State<'_, AppState>,
    endpoint: WebhookEndpoint,
    secret: Option<String>,
) -> Result<(), IpcErrorPayload> {
    // If secret provided, store in OS vault
    let mut updated_endpoint = endpoint;
    if let Some(sec) = secret {
        if !sec.trim().is_empty() {
            let secret_key = format!("webhook_{}", updated_endpoint.id);
            let secret_str = SecretString::new(sec);
            state
                .platform_vault
                .store(&secret_key, &secret_str)
                .await
                .map_err(IpcErrorPayload::from)?;
            updated_endpoint.vault_secret_key = Some(secret_key);
        }
    }

    let repo = WebhooksRepository::new(state.storage.clone());
    repo.insert_or_update_endpoint(&updated_endpoint)
        .map_err(IpcErrorPayload::from)
}

#[tauri::command]
pub async fn webhook_delete_endpoint(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, IpcErrorPayload> {
    let repo = WebhooksRepository::new(state.storage.clone());
    let secret_key = format!("webhook_{}", id);
    let _ = state.platform_vault.delete(&secret_key).await;
    repo.delete_endpoint(&id).map_err(IpcErrorPayload::from)
}

#[tauri::command]
pub async fn webhook_test_ping(
    state: State<'_, AppState>,
    endpoint_id: String,
) -> Result<u16, IpcErrorPayload> {
    let repo = WebhooksRepository::new(state.storage.clone());
    let endpoint = repo
        .get_endpoint(&endpoint_id)
        .map_err(IpcErrorPayload::from)?
        .ok_or_else(|| IpcErrorPayload {
            code: "NOT_FOUND".into(),
            message: format!("Endpoint '{endpoint_id}' not found"),
            recoverable: false,
        })?;

    let secret = if let Some(key) = &endpoint.vault_secret_key {
        state.platform_vault.retrieve(key).await.ok()
    } else {
        None
    };

    state
        .webhooks
        .test_ping(&endpoint, secret.as_ref())
        .await
        .map_err(|e| IpcErrorPayload {
            code: "DELIVERY_ERROR".into(),
            message: e.to_string(),
            recoverable: true,
        })
}

#[tauri::command]
pub async fn webhook_list_deliveries(
    state: State<'_, AppState>,
    endpoint_id: String,
    limit: Option<usize>,
) -> Result<Vec<WebhookDeliveryRecord>, IpcErrorPayload> {
    let repo = WebhooksRepository::new(state.storage.clone());
    repo.list_deliveries(&endpoint_id, limit.unwrap_or(50))
        .map_err(IpcErrorPayload::from)
}
