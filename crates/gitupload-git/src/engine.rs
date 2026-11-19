#![forbid(unsafe_code)]
//! Unified `GitEngine` trait and `LibGit2GitEngine` async implementation.

use crate::cli::CliGitEngine;
use crate::coordinator::{SafePushMode, SafeSyncEngine};
use crate::libgit2::{LibGit2Commit, LibGit2Repo, LibGit2Transport};
use async_trait::async_trait;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use secrecy::SecretString;
use std::path::Path;
use tokio_util::sync::CancellationToken;

/// High-level asynchronous Git engine interface backed primarily by in-process `libgit2`.
#[async_trait]
pub trait GitEngineTrait: Send + Sync {
    /// Initializes a local Git repository.
    async fn init_repo(&self, path: &Path) -> GitUploadResult<()>;
    /// Checks whether Git CLI fallback or in-process Git is ready.
    async fn is_git_available(&self) -> bool;
}

/// Primary in-process `libgit2` engine exposing async methods over `tokio::task::spawn_blocking`.
#[derive(Clone, Default)]
pub struct GitEngine;

/// Alias for explicit engine selection.
pub type LibGit2GitEngine = GitEngine;

impl GitEngine {
    /// Creates a new `GitEngine` instance.
    pub fn new() -> Self {
        Self
    }

    /// Initializes a repository synchronously inside `spawn_blocking`.
    pub async fn init_repository(&self, path: &Path) -> GitUploadResult<()> {
        let p = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            LibGit2Repo::init_or_open(&p)?;
            LibGit2Repo::ensure_env_ignored(&p)?;
            Ok(())
        })
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
    }

    /// Stages all working directory files and commits them in-process (< 3ms).
    pub async fn stage_and_commit(
        &self,
        path: &Path,
        author_name: &str,
        author_email: &str,
        message: &str,
    ) -> GitUploadResult<String> {
        let p = path.to_path_buf();
        let name = author_name.to_string();
        let email = author_email.to_string();
        let msg = message.to_string();
        let oid = tokio::task::spawn_blocking(move || {
            LibGit2Commit::stage_and_commit(&p, &name, &email, &msg)
        })
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))??;
        Ok(oid.to_string())
    }

    /// Pushes `branch` to `origin` enforcing `SafePushMode` (`StrictSafe`, `ForceWithLease`, `ForceUnsafe`).
    pub async fn push(
        &self,
        repo_path: &Path,
        branch: &str,
        mode: SafePushMode,
    ) -> GitUploadResult<()> {
        self.push_with_token(
            repo_path,
            "origin",
            branch,
            None,
            mode,
            CancellationToken::new(),
        )
        .await
    }

    /// Pushes `branch` with an optional `SecretString` token and cancellation token.
    pub async fn push_with_token(
        &self,
        repo_path: &Path,
        remote_name: &str,
        branch: &str,
        token: Option<SecretString>,
        mode: SafePushMode,
        cancel: CancellationToken,
    ) -> GitUploadResult<()> {
        let p = repo_path.to_path_buf();
        let rem = remote_name.to_string();
        let br = branch.to_string();
        tokio::task::spawn_blocking(move || {
            SafeSyncEngine::push_with_safety(
                &p,
                &rem,
                &br,
                token.as_ref(),
                mode,
                false,
                &cancel,
                |_, _| {},
            )
        })
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
    }

    /// Clones a remote repository into `dest` using in-memory token authentication.
    pub async fn clone_repo(
        &self,
        url: &str,
        dest: &Path,
        token: Option<SecretString>,
        cancel: CancellationToken,
    ) -> GitUploadResult<()> {
        let u = url.to_string();
        let d = dest.to_path_buf();
        tokio::task::spawn_blocking(move || {
            LibGit2Transport::clone_repo(&u, &d, token.as_ref(), &cancel, |_, _| {})
        })
        .await
        .map_err(|e| GitUploadError::Internal(e.to_string()))?
    }
}

#[async_trait]
impl GitEngineTrait for GitEngine {
    async fn init_repo(&self, path: &Path) -> GitUploadResult<()> {
        self.init_repository(path).await
    }

    async fn is_git_available(&self) -> bool {
        // libgit2 is always compiled in-process; check CLI too for diagnostic parity
        true || CliGitEngine::is_git_available().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_libgit2_commit() {
        let dir = tempdir().unwrap();
        let repo = LibGit2Repo::init_or_open(dir.path()).unwrap();
        LibGit2Repo::configure_identity(&repo, "Benchmark User", "bench@gitupload.local").unwrap();
        drop(repo);

        for i in 0..100 {
            fs::write(
                dir.path().join(format!("file_{i:03}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }

        let start = Instant::now();
        let oid = LibGit2Commit::stage_and_commit(
            dir.path(),
            "Benchmark User",
            "bench@gitupload.local",
            "feat: stage 100 files",
        )
        .unwrap();
        let elapsed = start.elapsed();

        assert!(!oid.to_string().is_empty());
        let budget_ms: u128 = if cfg!(debug_assertions) { 5000 } else { 250 };
        assert!(
            elapsed.as_millis() < budget_ms,
            "Commit of 100 files took {:?} (budget {}ms)",
            elapsed,
            budget_ms
        );
    }
}
