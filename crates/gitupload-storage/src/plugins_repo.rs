use crate::db::StoragePool;
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::plugin_manifest::{PluginCapabilities, UserPluginRecord};
use rusqlite::params;

pub struct PluginsRepository {
    pool: StoragePool,
}

impl PluginsRepository {
    pub fn new(pool: StoragePool) -> Self {
        Self { pool }
    }

    pub fn list_installed(&self) -> GitUploadResult<Vec<UserPluginRecord>> {
        let conn = self.pool.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, author, entrypoint, capabilities, is_enabled, installed_at
                 FROM user_plugins ORDER BY installed_at ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let caps_json: String = row.get(5)?;
                let installed_at_str: String = row.get(7)?;

                let capabilities: PluginCapabilities =
                    serde_json::from_str(&caps_json).unwrap_or_default();
                let installed_at = DateTime::parse_from_rfc3339(&installed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(UserPluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    author: row.get(3)?,
                    entrypoint: row.get(4)?,
                    capabilities,
                    is_enabled: row.get::<_, i32>(6)? == 1,
                    installed_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn get(&self, id: &str) -> GitUploadResult<Option<UserPluginRecord>> {
        let conn = self.pool.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, version, author, entrypoint, capabilities, is_enabled, installed_at
                 FROM user_plugins WHERE id = ?1",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![id], |row| {
                let caps_json: String = row.get(5)?;
                let installed_at_str: String = row.get(7)?;

                let capabilities: PluginCapabilities =
                    serde_json::from_str(&caps_json).unwrap_or_default();
                let installed_at = DateTime::parse_from_rfc3339(&installed_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(UserPluginRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    author: row.get(3)?,
                    entrypoint: row.get(4)?,
                    capabilities,
                    is_enabled: row.get::<_, i32>(6)? == 1,
                    installed_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| GitUploadError::Storage(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn register(&self, plugin: &UserPluginRecord) -> GitUploadResult<()> {
        let conn = self.pool.conn()?;
        let caps_json = serde_json::to_string(&plugin.capabilities)
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let installed_at_str = plugin.installed_at.to_rfc3339();

        conn.execute(
            "INSERT INTO user_plugins (id, name, version, author, entrypoint, capabilities, is_enabled, installed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                version = excluded.version,
                author = excluded.author,
                entrypoint = excluded.entrypoint,
                capabilities = excluded.capabilities,
                is_enabled = excluded.is_enabled",
            params![
                plugin.id,
                plugin.name,
                plugin.version,
                plugin.author,
                plugin.entrypoint,
                caps_json,
                if plugin.is_enabled { 1 } else { 0 },
                installed_at_str,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> GitUploadResult<()> {
        let conn = self.pool.conn()?;
        conn.execute(
            "UPDATE user_plugins SET is_enabled = ?1 WHERE id = ?2",
            params![if enabled { 1 } else { 0 }, id],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> GitUploadResult<bool> {
        let conn = self.pool.conn()?;
        let count = conn
            .execute("DELETE FROM user_plugins WHERE id = ?1", params![id])
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(count > 0)
    }
}
