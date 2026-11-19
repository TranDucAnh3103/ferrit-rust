//! SQLite BLOB vector storage for semantic commit embeddings and DevSecOps scan audit records.

use crate::db::StoragePool;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::SecurityAuditReport;
use rusqlite::params;

/// Persistence repository managing IEEE-754 `f32` embedding vectors and DevSecOps scan audits.
pub struct EmbeddingStore<'a> {
    storage: &'a StoragePool,
}

impl<'a> EmbeddingStore<'a> {
    /// Creates a new [`EmbeddingStore`] bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Serializes a slice of `f32` floats into a compact little-endian byte vector.
    pub fn f32_to_blob(vector: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(vector.len() * 4);
        for &val in vector {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }

    /// Deserializes a little-endian byte slice back into an `f32` vector safely.
    pub fn blob_to_f32(blob: &[u8]) -> Vec<f32> {
        blob.as_chunks::<4>()
            .0
            .iter()
            .map(|&arr| f32::from_le_bytes(arr))
            .collect()
    }

    /// Saves or updates a commit's embedding vector.
    pub fn save_embedding(
        &self,
        repo_path: &str,
        commit_oid: &str,
        summary: &str,
        vector: &[f32],
    ) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let blob = Self::f32_to_blob(vector);
        conn.execute(
            "INSERT OR REPLACE INTO commit_embeddings (repo_path, commit_oid, summary, embedding_blob, dimensions)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repo_path.trim(), commit_oid, summary, blob, vector.len() as i64],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Loads all indexed `(commit_oid, summary, vector)` entries for a repository.
    pub fn load_repo_embeddings(
        &self,
        repo_path: &str,
    ) -> GitUploadResult<Vec<(String, String, Vec<f32>)>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT commit_oid, summary, embedding_blob
                 FROM commit_embeddings
                 WHERE repo_path = ?1",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![repo_path.trim()], |row| {
                let oid: String = row.get(0)?;
                let summary: String = row.get(1)?;
                let blob: Vec<u8> = row.get(2)?;
                Ok((oid, summary, Self::blob_to_f32(&blob)))
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(results)
    }

    /// Persists a DevSecOps security audit summary record inside `security_scan_history`.
    pub fn record_security_scan(&self, report: &SecurityAuditReport) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO security_scan_history
                (scan_id, repo_path, total_findings, critical_count, high_count, passed, scan_duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                report.scan_id,
                report.repo_path,
                report.total_findings as i64,
                report.critical_count as i64,
                report.high_count as i64,
                report.passed,
                report.scan_duration_ms as i64,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }
}
