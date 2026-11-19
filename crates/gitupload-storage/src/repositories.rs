//! Strongly-typed relational query repositories over SQLite WAL storage.

use crate::db::StoragePool;
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{AppSettings, GitHubOrg, OrgMembershipRole, PushHistoryRecord};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

const APP_SETTINGS_JSON_KEY: &str = "app_settings_json";

/// Repository managing non-sensitive `AppSettings` key-value storage.
pub struct SettingsRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> SettingsRepository<'a> {
    /// Creates a new `SettingsRepository` bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Sets an arbitrary key-value pair in `app_settings`.
    pub fn set_raw(&self, key: &str, value: &str) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Reads a raw setting value by key.
    pub fn get_raw(&self, key: &str) -> GitUploadResult<Option<String>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| GitUploadError::Storage(e.to_string()))
    }

    /// Persists `AppSettings` JSON inside SQLite (contains zero secret tokens by design).
    pub fn save_app_settings(&self, settings: &AppSettings) -> GitUploadResult<()> {
        let json = serde_json::to_string(settings)?;
        self.set_raw(APP_SETTINGS_JSON_KEY, &json)
    }

    /// Loads `AppSettings` from SQLite, returning `AppSettings::default()` if absent.
    pub fn load_app_settings(&self) -> GitUploadResult<AppSettings> {
        match self.get_raw(APP_SETTINGS_JSON_KEY)? {
            Some(json) => Ok(serde_json::from_str(&json)?),
            None => Ok(AppSettings::default()),
        }
    }
}

/// Repository managing unlimited historical push audit records.
pub struct HistoryRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> HistoryRepository<'a> {
    /// Creates a new `HistoryRepository` bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Inserts or updates a push history record.
    pub fn save(&self, record: &PushHistoryRecord) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let id = if record.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            record.id.clone()
        };
        let pushed_at = record.created_at.to_rfc3339();
        conn.execute(
            "INSERT INTO push_history (
                id, account_id, repo_name, repo_url, local_path, commit_sha,
                commit_msg, files_count, status, error_message, is_force_with_lease,
                duration_ms, pushed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                repo_url = excluded.repo_url,
                commit_sha = excluded.commit_sha,
                status = excluded.status,
                error_message = excluded.error_message,
                duration_ms = excluded.duration_ms,
                pushed_at = excluded.pushed_at",
            params![
                id,
                record.account_id,
                record.repo_name,
                record.repo_url,
                record.local_path,
                record.commit_sha,
                record.commit_msg,
                record.files_count as i64,
                record.status,
                record.error_message,
                record.is_force_with_lease,
                record.duration_ms as i64,
                pushed_at,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Lists the most recent push history entries ordered newest-first.
    pub fn list_recent(&self, limit: usize) -> GitUploadResult<Vec<PushHistoryRecord>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT id, account_id, repo_name, repo_url, local_path, commit_sha,
                        commit_msg, files_count, status, error_message, is_force_with_lease,
                        duration_ms, pushed_at
                 FROM push_history
                 ORDER BY pushed_at DESC
                 LIMIT ?1",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| {
                let ts_str: String = row.get(12)?;
                let created_at = DateTime::parse_from_rfc3339(&ts_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(PushHistoryRecord {
                    id: row.get(0)?,
                    account_id: row.get(1)?,
                    repo_name: row.get(2)?,
                    repo_url: row.get(3)?,
                    local_path: row.get(4)?,
                    commit_sha: row.get(5)?,
                    commit_msg: row.get(6)?,
                    files_count: row.get::<_, i64>(7)?.max(0) as u64,
                    status: row.get(8)?,
                    error_message: row.get(9)?,
                    is_force_with_lease: row.get(10)?,
                    duration_ms: row.get::<_, i64>(11)?.max(0) as u64,
                    created_at,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut records = Vec::new();
        for item in rows {
            records.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(records)
    }

    /// Returns the most recent push record if available.
    pub fn get_last_pushed(&self) -> GitUploadResult<Option<PushHistoryRecord>> {
        Ok(self.list_recent(1)?.into_iter().next())
    }
}

/// Repository caching GitHub Organizations and membership permissions.
pub struct OrgRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> OrgRepository<'a> {
    /// Creates a new `OrgRepository` bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Replaces or updates cached organizations in SQLite.
    pub fn upsert_all(&self, orgs: &[GitHubOrg]) -> GitUploadResult<()> {
        let mut conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        for org in orgs {
            let role_str = match org.role {
                OrgMembershipRole::Admin => "admin",
                OrgMembershipRole::Member => "member",
            };
            tx.execute(
                "INSERT INTO organizations (login, id, avatar_url, description, is_admin, role, can_create_repo)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(login) DO UPDATE SET
                    id = excluded.id,
                    avatar_url = excluded.avatar_url,
                    description = excluded.description,
                    is_admin = excluded.is_admin,
                    role = excluded.role,
                    can_create_repo = excluded.can_create_repo",
                params![
                    org.login,
                    org.id as i64,
                    org.avatar_url,
                    org.description,
                    org.is_admin,
                    role_str,
                    org.can_create_repo,
                ],
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Lists all cached organizations ordered by login name.
    pub fn list(&self) -> GitUploadResult<Vec<GitHubOrg>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT login, id, avatar_url, description, is_admin, role, can_create_repo
                 FROM organizations ORDER BY login ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let role_str: String = row.get(5)?;
                let role = if role_str.eq_ignore_ascii_case("admin") {
                    OrgMembershipRole::Admin
                } else {
                    OrgMembershipRole::Member
                };
                Ok(GitHubOrg {
                    login: row.get(0)?,
                    id: row.get::<_, i64>(1)?.max(0) as u64,
                    avatar_url: row.get(2)?,
                    description: row.get(3)?,
                    is_admin: row.get(4)?,
                    role,
                    can_create_repo: row.get(6)?,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut orgs = Vec::new();
        for item in rows {
            orgs.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(orgs)
    }
}

/// Repository storing native Git hooks configuration overrides.
pub struct HooksConfigRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> HooksConfigRepository<'a> {
    /// Creates a new `HooksConfigRepository` bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Persists a hooks configuration entry.
    pub fn set(&self, key: &str, value: &str) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO hooks_config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Retrieves a hooks configuration entry by key.
    pub fn get(&self, key: &str) -> GitUploadResult<Option<String>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.query_row(
            "SELECT value FROM hooks_config WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| GitUploadError::Storage(e.to_string()))
    }
}
