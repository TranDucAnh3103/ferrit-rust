#![forbid(unsafe_code)]
//! Registered Tauri v2 IPC commands organized by domain.

pub mod ai;
pub mod analytics;
pub mod auth;
pub mod enterprise;
pub mod ghcli;
pub mod git;
pub mod github;
pub mod graph;
pub mod hooks;
pub mod hygiene;
pub mod marketplace;
pub mod merge;
pub mod mirror;
pub mod plugins;
pub mod security;
pub mod semantic;
pub mod ssh;
pub mod sync_daemon;
pub mod system;
pub mod updater;
pub mod webhooks;

pub use ai::*;
pub use analytics::*;
pub use auth::*;
pub use enterprise::*;
pub use ghcli::*;
pub use git::*;
pub use github::*;
pub use graph::*;
pub use hooks::*;
pub use hygiene::*;
pub use marketplace::*;
pub use merge::*;
pub use mirror::*;
pub use plugins::*;
pub use security::*;
pub use semantic::*;
pub use ssh::*;
pub use sync_daemon::*;
pub use system::*;
pub use updater::*;
pub use webhooks::*;

use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use std::path::{Component, Path, PathBuf};

/// Validates and canonicalizes an IPC filesystem path, rejecting traversal segments (`..`) and empty inputs.
pub(crate) fn validate_safe_path(raw: &str) -> Result<PathBuf, IpcErrorPayload> {
    let trimmed = raw.trim().trim_matches('"');
    if trimmed.is_empty() {
        return Err(GitUploadError::Config("Filesystem path cannot be empty".into()).into());
    }

    let candidate = Path::new(trimmed);
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(GitUploadError::SafetyGuard(format!(
            "Path traversal ('..') is forbidden in IPC path: '{trimmed}'"
        ))
        .into());
    }

    if candidate.exists() {
        std::fs::canonicalize(candidate).map_err(|e| GitUploadError::Io(e).into())
    } else {
        Ok(candidate.to_path_buf())
    }
}
