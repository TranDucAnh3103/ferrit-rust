use crate::db::StoragePool;
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::webhooks::{WebhookDeliveryRecord, WebhookEndpoint, WebhookTransport};
use rusqlite::params;
use std::str::FromStr;

pub struct WebhooksRepository {
    pool: StoragePool,
}

impl WebhooksRepository {
    pub fn new(pool: StoragePool) -> Self {
        Self { pool }
    }

    pub fn list_endpoints(&self) -> GitUploadResult<Vec<WebhookEndpoint>> {
        let conn = self.pool.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, transport, target_url, vault_secret_key, events, is_active, created_at
                 FROM webhook_endpoints ORDER BY created_at ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let transport_str: String = row.get(2)?;
                let events_str: String = row.get(5)?;
                let created_at_str: String = row.get(7)?;

                let transport =
                    WebhookTransport::from_str(&transport_str).unwrap_or(WebhookTransport::Generic);
                let events = events_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(WebhookEndpoint {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    transport,
                    target_url: row.get(3)?,
                    vault_secret_key: row.get(4)?,
                    events,
                    is_active: row.get::<_, i32>(6)? == 1,
                    created_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn get_endpoint(&self, id: &str) -> GitUploadResult<Option<WebhookEndpoint>> {
        let conn = self.pool.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, transport, target_url, vault_secret_key, events, is_active, created_at
                 FROM webhook_endpoints WHERE id = ?1",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let transport_str: String = row.get(2)?;
                let events_str: String = row.get(5)?;
                let created_at_str: String = row.get(7)?;

                let transport =
                    WebhookTransport::from_str(&transport_str).unwrap_or(WebhookTransport::Generic);
                let events = events_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(WebhookEndpoint {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    transport,
                    target_url: row.get(3)?,
                    vault_secret_key: row.get(4)?,
                    events,
                    is_active: row.get::<_, i32>(6)? == 1,
                    created_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| GitUploadError::Storage(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_or_update_endpoint(&self, endpoint: &WebhookEndpoint) -> GitUploadResult<()> {
        let conn = self.pool.conn()?;
        let events_str = endpoint.events.join(",");
        let created_at_str = endpoint.created_at.to_rfc3339();

        conn.execute(
            "INSERT INTO webhook_endpoints (id, name, transport, target_url, vault_secret_key, events, is_active, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                transport = excluded.transport,
                target_url = excluded.target_url,
                vault_secret_key = excluded.vault_secret_key,
                events = excluded.events,
                is_active = excluded.is_active",
            params![
                endpoint.id,
                endpoint.name,
                endpoint.transport.to_string(),
                endpoint.target_url,
                endpoint.vault_secret_key,
                events_str,
                if endpoint.is_active { 1 } else { 0 },
                created_at_str,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn delete_endpoint(&self, id: &str) -> GitUploadResult<bool> {
        let conn = self.pool.conn()?;
        let count = conn
            .execute("DELETE FROM webhook_endpoints WHERE id = ?1", params![id])
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn record_delivery(&self, delivery: &WebhookDeliveryRecord) -> GitUploadResult<()> {
        let conn = self.pool.conn()?;
        let delivered_at_str = delivery.delivered_at.to_rfc3339();

        conn.execute(
            "INSERT INTO webhook_deliveries (id, endpoint_id, event_type, status_code, attempt_count, duration_ms, error_message, delivered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                delivery.id,
                delivery.endpoint_id,
                delivery.event_type,
                delivery.status_code,
                delivery.attempt_count,
                delivery.duration_ms,
                delivery.error_message,
                delivered_at_str,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn list_deliveries(
        &self,
        endpoint_id: &str,
        limit: usize,
    ) -> GitUploadResult<Vec<WebhookDeliveryRecord>> {
        let conn = self.pool.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, endpoint_id, event_type, status_code, attempt_count, duration_ms, error_message, delivered_at
                 FROM webhook_deliveries
                 WHERE endpoint_id = ?1
                 ORDER BY delivered_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![endpoint_id, limit as i64], |row| {
                let delivered_at_str: String = row.get(7)?;
                let delivered_at = DateTime::parse_from_rfc3339(&delivered_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(WebhookDeliveryRecord {
                    id: row.get(0)?,
                    endpoint_id: row.get(1)?,
                    event_type: row.get(2)?,
                    status_code: row.get(3)?,
                    attempt_count: row.get(4)?,
                    duration_ms: row.get(5)?,
                    error_message: row.get(6)?,
                    delivered_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(results)
    }
}
