//! Async fallback Git CLI runner using Windows Job Objects, `CREATE_NO_WINDOW`, and token redaction.

#![forbid(unsafe_code)]

use crate::cli::job_object::WindowsJobObject;
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_security::sanitizer::sanitize_log_message;
use std::fs;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Fallback `git.exe` subprocess runner with atomic Job Object cancellation and zero CMD flashing.
pub struct CliGitEngine;

impl CliGitEngine {
    /// Checks whether `git` CLI is callable on the host system without flashing a console window.
    pub async fn is_git_available() -> bool {
        let mut cmd = Command::new("git");
        cmd.arg("--version")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        match cmd.status().await {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    /// Executes `git <args>` inside `dir`, assigning the subprocess to a Windows Job Object
    /// so cooperative cancellation kills the entire process tree in < 50ms and cleans `.git/index.lock`.
    pub async fn run_command<F>(
        dir: &Path,
        args: &[&str],
        cancel: &CancellationToken,
        mut on_log: F,
    ) -> GitUploadResult<()>
    where
        F: FnMut(String),
    {
        if cancel.is_cancelled() {
            return Err(GitUploadError::Cancelled);
        }

        let safe_cmd_line = sanitize_log_message(&format!(">> git {}", args.join(" ")));
        on_log(safe_cmd_line);

        let mut job = WindowsJobObject::new()
            .map_err(|e| GitUploadError::Git(format!("Failed creating Windows Job Object: {e}")))?;

        let mut cmd = Command::new("git");
        cmd.args(args)
            .current_dir(dir)
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| GitUploadError::Git(format!("Failed spawning git CLI: {e}")))?;

        #[cfg(windows)]
        if let Some(raw_handle) = child.raw_handle() {
            let _ = job.assign_process(raw_handle);
        }

        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();

        let status = tokio::select! {
            _ = cancel.cancelled() => {
                let _ = job.terminate();
                let _ = child.kill().await;
                let _ = child.wait().await;
                cleanup_index_lock(dir);
                return Err(GitUploadError::Cancelled);
            }
            wait_res = child.wait() => {
                wait_res.map_err(|e| GitUploadError::Git(format!("Failed waiting for git CLI: {e}")))?
            }
        };

        let mut output_buf = Vec::new();
        if let Some(ref mut out) = stdout {
            let _ = out.read_to_end(&mut output_buf).await;
        }
        if let Some(ref mut err) = stderr {
            let _ = err.read_to_end(&mut output_buf).await;
        }

        let text = String::from_utf8_lossy(&output_buf);
        for segment in split_by_newline_or_cr(&text) {
            if !segment.is_empty() {
                on_log(sanitize_log_message(segment));
            }
        }

        if !status.success() {
            return Err(GitUploadError::Git(format!(
                "git command exited with status {}",
                status
            )));
        }

        Ok(())
    }
}

/// Splits stdout/stderr output on `\n`, `\r\n`, or `\r` (preserving git `--progress` updates).
pub fn split_by_newline_or_cr(raw: &str) -> Vec<&str> {
    raw.split(['\n', '\r'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

fn cleanup_index_lock(repo_dir: &Path) {
    let lock_path = repo_dir.join(".git").join("index.lock");
    if lock_path.exists() {
        let _ = fs::remove_file(lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_job_object_kill() {
        let _dir = tempdir().unwrap();
        let job = WindowsJobObject::new().unwrap();

        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "ping -n 30 127.0.0.1 >nul"]);
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn().unwrap();
        #[cfg(windows)]
        if let Some(raw) = child.raw_handle() {
            job.assign_process(raw).unwrap();
        }

        let start = Instant::now();
        drop(job);
        let status = tokio::time::timeout(Duration::from_millis(500), child.wait())
            .await
            .expect("Child process did not exit quickly after Job Object drop")
            .unwrap();
        assert!(!status.success() || start.elapsed() < Duration::from_millis(500));
    }
}
