//! Pure-Rust Git hook validation engine (<2ms execution, zero Node.js/Python dependencies).

use crate::config::{HooksConfig, PreCommitConfig, PrePushConfig};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static RE_CONVENTIONAL: OnceLock<Option<Regex>> = OnceLock::new();
static RE_SECRET_SET: OnceLock<Option<RegexSet>> = OnceLock::new();

fn re_conventional() -> Option<&'static Regex> {
    RE_CONVENTIONAL
        .get_or_init(|| {
            Regex::new(
                r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-zA-Z0-9._/-]+\))?!?: .+",
            )
            .ok()
        })
        .as_ref()
}

fn re_secret_set() -> Option<&'static RegexSet> {
    RE_SECRET_SET
        .get_or_init(|| {
            RegexSet::new([
                r"(gh[pousr]_[A-Za-z0-9_]{20,255}|github_pat_[A-Za-z0-9_]{20,255})",
                r"AKIA[0-9A-Z]{16}",
                r"-----BEGIN (?:RSA |OPENSSH |EC |DSA )?PRIVATE KEY-----",
                r"(sk-[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{20,})",
            ])
            .ok()
        })
        .as_ref()
}

/// Detected secret finding in a staged file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretFinding {
    /// Relative path of the offending file.
    pub file_path: String,
    /// Kind of secret detected (`GitHub Token`, `AWS Access Key`, `Private Key`, `API Key`).
    pub kind: String,
}

/// Structured output returned from pre-commit safety scanning.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreCommitReport {
    /// Number of staged files inspected.
    pub files_scanned: usize,
    /// Blocked `.env` files detected.
    pub blocked_env_files: Vec<String>,
    /// Files exceeding `max_file_size_mb`.
    pub oversized_files: Vec<(String, u64)>,
    /// Secret patterns discovered in staged files.
    pub secret_findings: Vec<SecretFinding>,
}

/// Zero-dependency native Rust Git hooks runner.
pub struct HookRunner;

impl HookRunner {
    /// Validates a commit message against Conventional Commits 1.0.0 and header length rules.
    pub fn validate_commit_msg(msg: &str, max_len: usize) -> GitUploadResult<()> {
        let first_line = msg.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            return Err(GitUploadError::Hook(
                "Commit message cannot be empty. Use `<type>(<scope>): <description>`, e.g. `feat(ui): add dark theme`.".into(),
            ));
        }

        if first_line.len() > max_len {
            return Err(GitUploadError::Hook(format!(
                "Commit header exceeds maximum allowed length of {} chars (actual: {}). Shorten the summary line.",
                max_len,
                first_line.len()
            )));
        }

        let matched = match re_conventional() {
            Some(re) => re.is_match(first_line),
            None => false,
        };

        if !matched {
            return Err(GitUploadError::Hook(format!(
                "Commit message '{}' does not adhere to Conventional Commits 1.0.0. \
                 Expected `<type>(<scope>): <description>` with allowed types: \
                 feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert (e.g. `fix(auth): handle keyring timeout`).",
                first_line
            )));
        }

        Ok(())
    }

    /// Scans staged files for `.env` leaks, secret tokens, and oversized binary assets (> 50MB).
    pub fn scan_pre_commit(
        repo_root: &Path,
        staged_files: &[PathBuf],
        config: &PreCommitConfig,
    ) -> GitUploadResult<PreCommitReport> {
        let mut report = PreCommitReport::default();
        if !config.enabled {
            return Ok(report);
        }

        let max_bytes = (config.max_file_size_mb as u64).saturating_mul(1024 * 1024);

        for rel_or_abs in staged_files {
            let full_path = if rel_or_abs.is_absolute() {
                rel_or_abs.clone()
            } else {
                repo_root.join(rel_or_abs)
            };
            let display_path = rel_or_abs.to_string_lossy().replace('\\', "/");

            if config.block_env_files
                && is_sensitive_env_filename(&full_path)
                && full_path.is_file()
            {
                report.files_scanned += 1;
                report.blocked_env_files.push(display_path);
                continue;
            }

            let file = match File::open(&full_path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                continue;
            }
            report.files_scanned += 1;

            let size = metadata.len();
            if config.max_file_size_mb > 0 && size > max_bytes {
                report.oversized_files.push((display_path.clone(), size));
                continue;
            }

            if config.block_secrets && size <= 5 * 1024 * 1024 {
                if let Some(kind) = scan_open_file_for_secret(file, size as usize)? {
                    report.secret_findings.push(SecretFinding {
                        file_path: display_path,
                        kind,
                    });
                }
            }
        }

        if !report.blocked_env_files.is_empty() {
            return Err(GitUploadError::Hook(format!(
                "Pre-commit blocked unencrypted environment file(s): {}. Add `.env` to `.gitignore` and store secrets in Windows Keyring.",
                report.blocked_env_files.join(", ")
            )));
        }

        if !report.secret_findings.is_empty() {
            let details: Vec<String> = report
                .secret_findings
                .iter()
                .map(|f| format!("{} ({})", f.file_path, f.kind))
                .collect();
            return Err(GitUploadError::Hook(format!(
                "Pre-commit secret scanner detected sensitive credential(s) in staged file(s): {}. Remove secrets before committing.",
                details.join(", ")
            )));
        }

        if !report.oversized_files.is_empty() {
            let details: Vec<String> = report
                .oversized_files
                .iter()
                .map(|(p, bytes)| format!("{p} ({:.2} MB)", *bytes as f64 / (1024.0 * 1024.0)))
                .collect();
            return Err(GitUploadError::Hook(format!(
                "Pre-commit blocked oversized file(s) exceeding {} MB limit: {}. Enable Git LFS tracking (`*.ext filter=lfs diff=lfs merge=lfs -text`) in `.gitattributes`.",
                config.max_file_size_mb,
                details.join(", ")
            )));
        }

        Ok(report)
    }

    /// Checks whether pushing directly to `target_branch` is prohibited by branch protection config.
    pub fn validate_pre_push(target_branch: &str, config: &PrePushConfig) -> GitUploadResult<()> {
        if !config.enabled {
            return Ok(());
        }

        let normalized = target_branch
            .trim()
            .strip_prefix("refs/heads/")
            .unwrap_or(target_branch.trim());

        let is_protected = config
            .prevent_direct_push_branches
            .iter()
            .any(|b| b.eq_ignore_ascii_case(normalized));

        if is_protected {
            return Err(GitUploadError::Hook(format!(
                "Pre-push guard blocked direct push to protected branch '{}'. Create a feature branch (e.g. `feat/my-change`) and open a Pull Request instead.",
                normalized
            )));
        }

        Ok(())
    }

    /// Runs the full native pre-flight hook suite (`commit-msg` + `pre-commit` + optional `pre-push`).
    pub fn validate_all(
        repo_root: &Path,
        staged_files: &[PathBuf],
        commit_msg: &str,
        target_branch: Option<&str>,
        config: &HooksConfig,
    ) -> GitUploadResult<PreCommitReport> {
        if config.commit_msg.enabled && config.commit_msg.enforce_conventional_commits {
            Self::validate_commit_msg(commit_msg, config.commit_msg.max_header_length)?;
        }

        let report = Self::scan_pre_commit(repo_root, staged_files, &config.pre_commit)?;

        if let Some(branch) = target_branch {
            Self::validate_pre_push(branch, &config.pre_push)?;
        }

        Ok(report)
    }
}

fn is_sensitive_env_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    if lower == ".env.example" || lower == ".env.template" || lower == ".env.sample" {
        return false;
    }
    lower == ".env" || lower.starts_with(".env.") || lower.ends_with(".env")
}

fn scan_open_file_for_secret(mut file: File, hint_size: usize) -> GitUploadResult<Option<String>> {
    let mut buf = Vec::with_capacity(hint_size.min(5 * 1024 * 1024));
    // Cap buffered read at 5 MiB (enforced by caller as well)
    file.by_ref().take(5 * 1024 * 1024).read_to_end(&mut buf)?;

    // Skip binary files containing NUL bytes
    if buf.contains(&0) {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&buf);
    if let Some(set) = re_secret_set() {
        let matches = set.matches(&text);
        if matches.matched(0) {
            return Ok(Some("GitHub Token".into()));
        }
        if matches.matched(1) {
            return Ok(Some("AWS Access Key".into()));
        }
        if matches.matched(2) {
            return Ok(Some("Private Key".into()));
        }
        if matches.matched(3) {
            return Ok(Some("Cloud API Key".into()));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_conventional() {
        assert!(HookRunner::validate_commit_msg("feat(auth): add keyring support", 72).is_ok());
        assert!(
            HookRunner::validate_commit_msg("fix!: handle empty repository edge case", 72).is_ok()
        );
        assert!(HookRunner::validate_commit_msg("chore: update workspace deps", 72).is_ok());
    }

    #[test]
    fn test_invalid_msg() {
        let err = HookRunner::validate_commit_msg("fixed bugs", 72).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Conventional Commits"));
        assert!(msg.contains("feat"));

        let long_msg = format!("feat(core): {}", "a".repeat(80));
        let err_len = HookRunner::validate_commit_msg(&long_msg, 72).unwrap_err();
        assert!(err_len.to_string().contains("maximum allowed length"));
    }

    #[test]
    fn test_pre_commit_blocks_secrets_and_env() {
        let dir = tempdir().unwrap();
        let env_file = dir.path().join(".env");
        fs::write(&env_file, "PORT=8080\n").unwrap();

        let cfg = PreCommitConfig {
            enabled: true,
            block_secrets: true,
            max_file_size_mb: 50,
            block_env_files: true,
        };

        let env_err =
            HookRunner::scan_pre_commit(dir.path(), &[PathBuf::from(".env")], &cfg).unwrap_err();
        assert!(env_err.to_string().contains(".env"));

        let code_file = dir.path().join("config.rs");
        fs::write(
            &code_file,
            "const TOKEN: &str = \"ghp_1234567890abcdef1234567890abcdef12\";",
        )
        .unwrap();

        let sec_err = HookRunner::scan_pre_commit(dir.path(), &[PathBuf::from("config.rs")], &cfg)
            .unwrap_err();
        assert!(sec_err.to_string().contains("GitHub Token"));
    }

    #[test]
    fn test_pre_commit_blocks_oversized_file() {
        let dir = tempdir().unwrap();
        let big_file = dir.path().join("bundle.bin");
        fs::write(&big_file, vec![1u8; 2 * 1024 * 1024]).unwrap();

        let cfg = PreCommitConfig {
            enabled: true,
            block_secrets: true,
            max_file_size_mb: 1,
            block_env_files: true,
        };

        let err = HookRunner::scan_pre_commit(dir.path(), &[PathBuf::from("bundle.bin")], &cfg)
            .unwrap_err();
        assert!(err.to_string().contains("Git LFS"));
    }

    #[test]
    fn test_pre_push_blocks_protected_branch() {
        let cfg = PrePushConfig {
            enabled: true,
            prevent_direct_push_branches: vec!["main".into(), "master".into()],
        };
        assert!(HookRunner::validate_pre_push("refs/heads/main", &cfg).is_err());
        assert!(HookRunner::validate_pre_push("master", &cfg).is_err());
        assert!(HookRunner::validate_pre_push("feat/git-graph-dag", &cfg).is_ok());
    }
}
