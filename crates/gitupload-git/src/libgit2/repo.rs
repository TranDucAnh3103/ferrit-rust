//! In-process `libgit2` repository initialization, `.gitignore` safety guard, status, and diff.

use git2::{DiffFormat, DiffOptions, Repository, StatusOptions};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

const SENSITIVE_IGNORE_PATTERNS: &[&str] = &[
    ".env",
    ".env.*",
    "*.key",
    "*.pem",
    "id_rsa",
    "secrets.json",
    ".DS_Store",
    "Thumbs.db",
];

/// Repository lifecycle and inspection helpers backed by `libgit2`.
pub struct LibGit2Repo;

impl LibGit2Repo {
    /// Opens an existing Git repository or initializes a new one with `main` as initial branch
    /// and disables background `gc.auto` to prevent Windows file-locking.
    pub fn init_or_open(path: &Path) -> GitUploadResult<Repository> {
        let repo = if path.join(".git").exists() {
            Repository::open(path)
                .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?
        } else {
            let mut opts = git2::RepositoryInitOptions::new();
            opts.initial_head("main");
            Repository::init_opts(path, &opts)
                .map_err(|e| GitUploadError::Git(format!("Failed to init repository: {e}")))?
        };

        if let Ok(mut cfg) = repo.config() {
            let _ = cfg.set_i32("gc.auto", 0);
        }

        Ok(repo)
    }

    /// Ensures `user.name` and `user.email` are configured on the local repository if provided.
    pub fn configure_identity(
        repo: &Repository,
        author_name: &str,
        author_email: &str,
    ) -> GitUploadResult<()> {
        let mut cfg = repo
            .config()
            .map_err(|e| GitUploadError::Git(format!("Failed to open git config: {e}")))?;
        if !author_name.trim().is_empty() {
            cfg.set_str("user.name", author_name.trim())
                .map_err(|e| GitUploadError::Git(format!("Failed to set user.name: {e}")))?;
        }
        if !author_email.trim().is_empty() {
            cfg.set_str("user.email", author_email.trim())
                .map_err(|e| GitUploadError::Git(format!("Failed to set user.email: {e}")))?;
        }
        Ok(())
    }

    /// Guarantees `.gitignore` blocks `.env` and sensitive key patterns prior to staging.
    pub fn ensure_env_ignored(dir: &Path) -> GitUploadResult<()> {
        let gitignore_path = dir.join(".gitignore");
        if !gitignore_path.exists() {
            let content = format!("{}\n", SENSITIVE_IGNORE_PATTERNS.join("\n"));
            fs::write(&gitignore_path, content)?;
            return Ok(());
        }

        let existing = fs::read_to_string(&gitignore_path)?;
        let existing_lines: Vec<&str> = existing.lines().map(str::trim).collect();
        let mut missing = Vec::new();
        for pat in SENSITIVE_IGNORE_PATTERNS {
            if !existing_lines.contains(pat) {
                missing.push(*pat);
            }
        }

        if !missing.is_empty() {
            let mut file = OpenOptions::new().append(true).open(&gitignore_path)?;
            if !existing.is_empty() && !existing.ends_with('\n') {
                writeln!(file)?;
            }
            for pat in missing {
                writeln!(file, "{pat}")?;
            }
        }

        Ok(())
    }

    /// Lists modified, staged, or untracked paths inside the working tree.
    pub fn status_files(repo_path: &Path) -> GitUploadResult<Vec<String>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?;
        let mut opts = StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true);
        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| GitUploadError::Git(format!("Failed to query status: {e}")))?;

        let mut files = Vec::new();
        for entry in statuses.iter() {
            if let Some(p) = entry.path() {
                files.push(p.to_string());
            }
        }
        Ok(files)
    }

    /// Computes a unified diff string comparing `HEAD` (or empty tree) to index/working directory.
    pub fn diff_workdir(repo_path: &Path) -> GitUploadResult<String> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {e}")))?;
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());

        let diff = repo
            .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_opts))
            .map_err(|e| GitUploadError::Git(format!("Failed to compute diff: {e}")))?;

        let mut out = String::new();
        diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
            let origin = line.origin();
            if origin == '+' || origin == '-' || origin == ' ' {
                out.push(origin);
            }
            out.push_str(&String::from_utf8_lossy(line.content()));
            true
        })
        .map_err(|e| GitUploadError::Git(format!("Failed to format diff: {e}")))?;

        Ok(out)
    }
}
