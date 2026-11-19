#![forbid(unsafe_code)]
//! Cryptographic commit signing configuration inspector and GPG/SSH `commit_signed` bridge.

use git2::{Oid, Repository, Signature};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Commit signing configuration resolved from local/global Git config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SigningConfig {
    /// True when `commit.gpgsign = true`.
    pub enabled: bool,
    /// Signing format (`"openpgp"` or `"ssh"`).
    pub format: String,
    /// Configured `user.signingkey` identifier or key path.
    pub signing_key: Option<String>,
}

/// Manages GPG and SSH cryptographic commit signing via `git2::Repository::commit_signed`.
pub struct CommitSigner;

impl CommitSigner {
    /// Inspects repository and global Git config for `commit.gpgsign`, `gpg.format`, and `user.signingkey`.
    pub fn inspect_config(repo_path: &Path) -> GitUploadResult<SigningConfig> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repo: {e}")))?;
        let cfg = repo
            .config()
            .map_err(|e| GitUploadError::Git(format!("Failed reading git config: {e}")))?;

        let enabled = cfg.get_bool("commit.gpgsign").unwrap_or(false);
        let format = cfg
            .get_string("gpg.format")
            .unwrap_or_else(|_| "openpgp".to_string());
        let signing_key = cfg.get_string("user.signingkey").ok();

        Ok(SigningConfig {
            enabled,
            format,
            signing_key,
        })
    }

    /// Creates a cryptographically signed commit using the configured GPG or SSH key.
    pub fn create_signed_commit(
        repo_path: &Path,
        author_name: &str,
        author_email: &str,
        message: &str,
    ) -> GitUploadResult<Oid> {
        let signing = Self::inspect_config(repo_path)?;
        let key = signing.signing_key.ok_or_else(|| {
            GitUploadError::Git("Cannot sign commit: `user.signingkey` is not configured".into())
        })?;

        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed opening repo: {e}")))?;
        let mut index = repo
            .index()
            .map_err(|e| GitUploadError::Git(format!("Failed opening index: {e}")))?;
        let tree_id = index
            .write_tree()
            .map_err(|e| GitUploadError::Git(format!("Failed writing tree: {e}")))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| GitUploadError::Git(e.to_string()))?;

        let sig = Signature::now(author_name, author_email)
            .map_err(|e| GitUploadError::Git(format!("Invalid signature: {e}")))?;

        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit<'_>> = parent_commit.iter().collect();

        let commit_buf = repo
            .commit_create_buffer(&sig, &sig, message, &tree, &parents)
            .map_err(|e| GitUploadError::Git(format!("Failed building commit buffer: {e}")))?;
        let payload = commit_buf
            .as_str()
            .ok_or_else(|| GitUploadError::Git("Commit buffer is not valid UTF-8".into()))?;

        let signature_block = invoke_signing_tool(&signing.format, &key, payload)?;
        let oid = repo
            .commit_signed(payload, &signature_block, Some("gpgsig"))
            .map_err(|e| GitUploadError::Git(format!("Failed writing signed commit: {e}")))?;

        repo.head()
            .and_then(|mut head| head.set_target(oid, message))
            .map_err(|e| GitUploadError::Git(format!("Failed advancing HEAD: {e}")))?;

        Ok(oid)
    }
}

fn invoke_signing_tool(format: &str, key: &str, payload: &str) -> GitUploadResult<String> {
    let mut cmd = if format.eq_ignore_ascii_case("ssh") {
        let mut c = Command::new("ssh-keygen");
        c.args(["-Y", "sign", "-f", key, "-n", "git"]);
        c
    } else {
        let mut c = Command::new("gpg");
        c.args(["--status-fd=2", "-bsau", key]);
        c
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| GitUploadError::Git(format!("Failed spawning signing binary: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(payload.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(GitUploadError::Git(format!(
            "Cryptographic signing failed: {}",
            err.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
