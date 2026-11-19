//! SQLite persistence repositories for multi-remote mirroring targets and managed SSH key descriptors.

use crate::db::StoragePool;
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{
    RemoteEndpoint, RemoteProviderType, SshKeyAlgorithm, SshKeyDescriptor, SyncPolicy,
};
use rusqlite::{params, OptionalExtension};

/// Persistence repository for per-repo multi-remote mirror targets (`repository_remotes`).
pub struct RemoteRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> RemoteRepository<'a> {
    /// Creates a new [`RemoteRepository`] bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    fn provider_to_str(provider: &RemoteProviderType) -> &'static str {
        match provider {
            RemoteProviderType::GitHub => "github",
            RemoteProviderType::GitLab => "gitlab",
            RemoteProviderType::Bitbucket => "bitbucket",
            RemoteProviderType::Gitea => "gitea",
            RemoteProviderType::Custom => "custom",
        }
    }

    fn provider_from_str(raw: &str) -> RemoteProviderType {
        match raw.trim().to_ascii_lowercase().as_str() {
            "github" => RemoteProviderType::GitHub,
            "gitlab" => RemoteProviderType::GitLab,
            "bitbucket" => RemoteProviderType::Bitbucket,
            "gitea" => RemoteProviderType::Gitea,
            _ => RemoteProviderType::Custom,
        }
    }

    fn policy_to_str(policy: &SyncPolicy) -> &'static str {
        match policy {
            SyncPolicy::Bidirectional => "bidirectional",
            SyncPolicy::PushOnly => "push_only",
            SyncPolicy::BackupMirror => "backup_mirror",
        }
    }

    fn policy_from_str(raw: &str) -> SyncPolicy {
        match raw.trim().to_ascii_lowercase().as_str() {
            "push_only" => SyncPolicy::PushOnly,
            "backup_mirror" => SyncPolicy::BackupMirror,
            _ => SyncPolicy::Bidirectional,
        }
    }

    /// Inserts or updates a remote target configuration for `repo_path`.
    pub fn upsert_remote(&self, repo_path: &str, remote: &RemoteEndpoint) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO repository_remotes
                (repo_path, remote_name, remote_url, provider, sync_policy, auth_account, is_enabled, timeout_seconds, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP)
             ON CONFLICT(repo_path, remote_name) DO UPDATE SET
                remote_url = excluded.remote_url,
                provider = excluded.provider,
                sync_policy = excluded.sync_policy,
                auth_account = excluded.auth_account,
                is_enabled = excluded.is_enabled,
                timeout_seconds = excluded.timeout_seconds,
                updated_at = CURRENT_TIMESTAMP",
            params![
                repo_path.trim(),
                remote.name.trim(),
                remote.url.trim(),
                Self::provider_to_str(&remote.provider),
                Self::policy_to_str(&remote.sync_policy),
                remote.auth_account.trim(),
                remote.is_enabled,
                remote.timeout_seconds as i64,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Lists all configured remotes for `repo_path`.
    pub fn list_remotes(&self, repo_path: &str) -> GitUploadResult<Vec<RemoteEndpoint>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT remote_name, remote_url, provider, sync_policy, auth_account, is_enabled, timeout_seconds
                 FROM repository_remotes
                 WHERE repo_path = ?1
                 ORDER BY id ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![repo_path.trim()], |row| {
                let provider_raw: String = row.get(2)?;
                let policy_raw: String = row.get(3)?;
                let timeout_raw: i64 = row.get(6)?;
                Ok(RemoteEndpoint {
                    name: row.get(0)?,
                    url: row.get(1)?,
                    provider: Self::provider_from_str(&provider_raw),
                    sync_policy: Self::policy_from_str(&policy_raw),
                    auth_account: row.get(4)?,
                    is_enabled: row.get(5)?,
                    timeout_seconds: timeout_raw.max(1) as u32,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(list)
    }

    /// Removes a remote configuration entry for `repo_path`.
    pub fn delete_remote(&self, repo_path: &str, remote_name: &str) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "DELETE FROM repository_remotes WHERE repo_path = ?1 AND remote_name = ?2",
            params![repo_path.trim(), remote_name.trim()],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }
}

/// Persistence repository storing non-sensitive SSH public key metadata (`ssh_keys`).
pub struct SshKeyRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> SshKeyRepository<'a> {
    /// Creates a new [`SshKeyRepository`] bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    fn alg_to_str(alg: SshKeyAlgorithm) -> &'static str {
        match alg {
            SshKeyAlgorithm::Ed25519 => "ed25519",
            SshKeyAlgorithm::Rsa4096 => "rsa4096",
        }
    }

    fn alg_from_str(raw: &str) -> SshKeyAlgorithm {
        match raw.trim().to_ascii_lowercase().as_str() {
            "rsa4096" => SshKeyAlgorithm::Rsa4096,
            _ => SshKeyAlgorithm::Ed25519,
        }
    }

    /// Inserts or updates a managed SSH key descriptor (public metadata only).
    pub fn upsert_key(&self, desc: &SshKeyDescriptor) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO ssh_keys
                (key_id, title, algorithm, public_key, fingerprint_sha256, created_at, is_registered_on_github)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(key_id) DO UPDATE SET
                title = excluded.title,
                public_key = excluded.public_key,
                fingerprint_sha256 = excluded.fingerprint_sha256,
                is_registered_on_github = excluded.is_registered_on_github",
            params![
                desc.key_id,
                desc.title,
                Self::alg_to_str(desc.algorithm),
                desc.public_key,
                desc.fingerprint_sha256,
                desc.created_at.to_rfc3339(),
                desc.is_registered_on_github,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Marks an existing SSH key as uploaded to GitHub.
    pub fn mark_registered_on_github(&self, key_id: &str, registered: bool) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "UPDATE ssh_keys SET is_registered_on_github = ?2 WHERE key_id = ?1",
            params![key_id, registered],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Fetches a single SSH key descriptor by ID.
    pub fn get_key(&self, key_id: &str) -> GitUploadResult<Option<SshKeyDescriptor>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.query_row(
            "SELECT key_id, title, algorithm, public_key, fingerprint_sha256, created_at, is_registered_on_github
             FROM ssh_keys WHERE key_id = ?1",
            params![key_id],
            |row| {
                let alg_raw: String = row.get(2)?;
                let created_raw: String = row.get(5)?;
                let created_at = DateTime::parse_from_rfc3339(&created_raw)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(SshKeyDescriptor {
                    key_id: row.get(0)?,
                    title: row.get(1)?,
                    algorithm: Self::alg_from_str(&alg_raw),
                    public_key: row.get(3)?,
                    fingerprint_sha256: row.get(4)?,
                    created_at,
                    is_registered_on_github: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|e| GitUploadError::Storage(e.to_string()))
    }

    /// Lists all managed SSH key descriptors ordered newest first.
    pub fn list_keys(&self) -> GitUploadResult<Vec<SshKeyDescriptor>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT key_id, title, algorithm, public_key, fingerprint_sha256, created_at, is_registered_on_github
                 FROM ssh_keys ORDER BY created_at DESC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let alg_raw: String = row.get(2)?;
                let created_raw: String = row.get(5)?;
                let created_at = DateTime::parse_from_rfc3339(&created_raw)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(SshKeyDescriptor {
                    key_id: row.get(0)?,
                    title: row.get(1)?,
                    algorithm: Self::alg_from_str(&alg_raw),
                    public_key: row.get(3)?,
                    fingerprint_sha256: row.get(4)?,
                    created_at,
                    is_registered_on_github: row.get(6)?,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(list)
    }

    /// Removes SSH key metadata record by ID.
    pub fn delete_key(&self, key_id: &str) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute("DELETE FROM ssh_keys WHERE key_id = ?1", params![key_id])
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }
}
