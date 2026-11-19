//! Pooled SQLite connection manager enforcing WAL mode, normal sync, and foreign key integrity.

use crate::schema::{
    INITIAL_SCHEMA, SCHEMA_V2_REMOTES_SSH, SCHEMA_V3_ANALYTICS, SCHEMA_V4_EMBEDDINGS_SECURITY,
    SCHEMA_V5_PLUGINS_WEBHOOKS,
};
use directories::ProjectDirs;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use r2d2::{CustomizeConnection, Pool};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// Connection pool alias for SQLite connections.
pub type DbPool = Pool<SqliteConnectionManager>;

#[derive(Debug, Clone, Copy)]
struct ConnectionPragmas {
    is_memory: bool,
}

impl CustomizeConnection<Connection, rusqlite::Error> for ConnectionPragmas {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        if !self.is_memory {
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
            )
        } else {
            conn.execute_batch("PRAGMA foreign_keys = ON;")
        }
    }
}

/// Thread-safe embedded SQLite persistence pool.
#[derive(Clone)]
pub struct StoragePool {
    pool: DbPool,
    db_path: PathBuf,
    is_memory: bool,
}

impl StoragePool {
    /// Initializes the database inside the platform application data directory (`%APPDATA%/ferrit/FERRIT/data/ferrit.db`).
    pub fn init_default() -> GitUploadResult<Self> {
        let proj_dirs = ProjectDirs::from("com", "ferrit", "FERRIT")
            .or_else(|| ProjectDirs::from("com", "stephen", "GitUpload"))
            .ok_or_else(|| GitUploadError::Storage("Cannot determine AppData dir".into()))?;
        let data_dir = proj_dirs.data_dir();
        fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("ferrit.db");

        // Migrate legacy database if it exists and new db does not
        if !db_path.exists() {
            if let Some(old_dirs) = ProjectDirs::from("com", "stephen", "GitUpload") {
                let old_db = old_dirs.data_dir().join("gitupload.db");
                if old_db.exists() {
                    let _ = fs::copy(&old_db, &db_path);
                }
            }
        }

        Self::new(db_path)
    }

    /// Initializes a pooled SQLite database at `db_path` with WAL mode enabled (pool size = 8).
    pub fn new(db_path: PathBuf) -> GitUploadResult<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let manager = SqliteConnectionManager::file(&db_path);
        let pool = Pool::builder()
            .max_size(8)
            .connection_customizer(Box::new(ConnectionPragmas { is_memory: false }))
            .build(manager)
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        {
            let conn = pool
                .get()
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(INITIAL_SCHEMA)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V2_REMOTES_SSH)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V3_ANALYTICS)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V4_EMBEDDINGS_SECURITY)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V5_PLUGINS_WEBHOOKS)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch("PRAGMA user_version = 5;")
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        }

        Ok(Self {
            pool,
            db_path,
            is_memory: false,
        })
    }

    /// Initializes an isolated in-memory SQLite pool for fast unit testing.
    pub fn in_memory() -> GitUploadResult<Self> {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder()
            .max_size(1)
            .connection_customizer(Box::new(ConnectionPragmas { is_memory: true }))
            .build(manager)
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        {
            let conn = pool
                .get()
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(INITIAL_SCHEMA)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V2_REMOTES_SSH)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V3_ANALYTICS)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V4_EMBEDDINGS_SECURITY)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch(SCHEMA_V5_PLUGINS_WEBHOOKS)
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            conn.execute_batch("PRAGMA user_version = 5;")
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        }

        Ok(Self {
            pool,
            db_path: PathBuf::from(":memory:"),
            is_memory: true,
        })
    }

    /// Returns a pooled database connection.
    pub fn conn(&self) -> GitUploadResult<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))
    }

    /// Returns a reference to the underlying `r2d2` connection pool.
    pub fn get_pool(&self) -> &DbPool {
        &self.pool
    }

    /// Returns the filesystem path of the SQLite database.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Queries the active SQLite `journal_mode` pragma (e.g. `"wal"`).
    pub fn journal_mode(&self) -> GitUploadResult<String> {
        let conn = self
            .pool
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(mode)
    }

    /// Returns true if this pool is backed by an in-memory SQLite instance.
    pub fn is_in_memory(&self) -> bool {
        self.is_memory
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::HistoryRepository;
    use chrono::Utc;
    use gitupload_core::models::PushHistoryRecord;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_wal() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("wal_test.db");
        let pool = StoragePool::new(db_path).unwrap();

        let mode = pool.journal_mode().unwrap();
        assert_eq!(mode.to_lowercase(), "wal");

        let history_repo = HistoryRepository::new(&pool);
        let record = PushHistoryRecord {
            id: "rec-1".into(),
            account_id: "default".into(),
            repo_name: "session01-ex01-homework".into(),
            repo_url: "https://github.com/user/session01-ex01-homework".into(),
            local_path: "F:/Repos/session01-ex01".into(),
            commit_sha: Some("abcdef1234567".into()),
            commit_msg: "feat(hw): init".into(),
            files_count: 3,
            status: "SUCCESS".into(),
            error_message: None,
            is_force_with_lease: true,
            duration_ms: 120,
            created_at: Utc::now(),
        };

        let start = Instant::now();
        history_repo.save(&record).unwrap();
        let rows = history_repo.list_recent(10).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].repo_name, "session01-ex01-homework");
        assert!(
            elapsed.as_millis() < 100,
            "SQLite WAL write+read took {:?}",
            elapsed
        );
    }
}
