//! GitHub CLI (`gh`) detection, version inspection, SHA256 verification, and sandbox extraction.

use directories::{BaseDirs, ProjectDirs};
use gitupload_core::errors::{GhCliError, GhCliResult};
use gitupload_core::models::GhCliStatus;
use sha2::{Digest, Sha256};
use std::env;
use std::fs::{self, File};
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use tracing::info;
use zip::ZipArchive;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Official GitHub CLI repository (`cli/cli`).
pub const GH_OFFICIAL_REPO: &str = "cli/cli";

/// Manages `gh` binary discovery, portable sandboxing (`%APPDATA%/rust-gitupload/bin/`), and SHA256 validation.
#[derive(Clone, Debug)]
pub struct GhLifecycleManager {
    sandbox_dir: PathBuf,
}

impl Default for GhLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GhLifecycleManager {
    /// Creates a manager using the standard non-admin user sandbox directory.
    pub fn new() -> Self {
        let sandbox_dir = BaseDirs::new()
            .map(|b| b.config_dir().join("rust-gitupload").join("bin"))
            .or_else(|| {
                ProjectDirs::from("com", "stephen", "GitUpload").map(|p| p.data_dir().join("bin"))
            })
            .unwrap_or_else(|| PathBuf::from("./bin"));
        Self { sandbox_dir }
    }

    /// Creates a manager bound to an explicit sandbox directory (used in tests).
    pub fn with_sandbox_dir(sandbox_dir: PathBuf) -> Self {
        Self { sandbox_dir }
    }

    /// Returns the sandbox directory where portable `gh.exe` is stored.
    pub fn sandbox_dir(&self) -> &Path {
        &self.sandbox_dir
    }

    /// Resolves `gh` executable path: checks app sandbox first, then system `PATH`.
    pub fn resolve_binary_path(&self) -> Option<PathBuf> {
        let exe_name = if cfg!(windows) { "gh.exe" } else { "gh" };
        let sandboxed = self.sandbox_dir.join(exe_name);
        if sandboxed.is_file() {
            return Some(sandboxed);
        }

        find_in_path(exe_name)
    }

    /// Convenience static detector matching initial skeleton API.
    pub fn detect_gh() -> Option<PathBuf> {
        Self::new().resolve_binary_path()
    }

    /// Returns the default sandbox `bin` directory path.
    pub fn get_sandbox_bin_dir() -> Option<PathBuf> {
        BaseDirs::new().map(|b| b.config_dir().join("rust-gitupload").join("bin"))
    }

    /// Inspects current `gh` installation status and semantic version.
    pub async fn check_status(&self) -> GhCliStatus {
        if let Some(path) = self.resolve_binary_path() {
            let is_sandboxed = path.starts_with(&self.sandbox_dir);
            let version = self.query_version(&path).await.ok();
            GhCliStatus {
                is_installed: true,
                is_sandboxed,
                version,
                binary_path: Some(path.to_string_lossy().to_string()),
            }
        } else {
            GhCliStatus {
                is_installed: false,
                is_sandboxed: false,
                version: None,
                binary_path: None,
            }
        }
    }

    /// Executes `gh --version` silently and parses the version token.
    pub async fn query_version(&self, binary: &Path) -> GhCliResult<String> {
        let mut cmd = tokio::process::Command::new(binary);
        cmd.arg("--version");
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = cmd
            .output()
            .await
            .map_err(|e| GhCliError::Process(e.to_string()))?;

        if !output.status.success() {
            return Err(GhCliError::Process("Failed to execute gh --version".into()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("").trim();
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() >= 3 {
            Ok(parts[2].to_string())
        } else {
            Ok(first_line.to_string())
        }
    }

    /// Verifies cryptographic SHA256 digest of downloaded release archive bytes.
    pub fn verify_checksum(bytes: &[u8], expected_sha256: &str) -> GhCliResult<()> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hex::encode(hasher.finalize());
        if digest.eq_ignore_ascii_case(expected_sha256.trim()) {
            Ok(())
        } else {
            Err(GhCliError::ChecksumMismatch {
                expected: expected_sha256.trim().to_string(),
                actual: digest,
            })
        }
    }

    /// Extracts `gh.exe` from official portable release zip bytes into the non-admin sandbox directory.
    pub fn extract_gh_exe(&self, zip_bytes: &[u8]) -> GhCliResult<PathBuf> {
        fs::create_dir_all(&self.sandbox_dir)
            .map_err(|e| GhCliError::Io(format!("Cannot create sandbox directory: {e}")))?;

        let target_name = if cfg!(windows) { "gh.exe" } else { "gh" };
        let target_exe = self.sandbox_dir.join(target_name);
        let reader = Cursor::new(zip_bytes);
        let mut archive =
            ZipArchive::new(reader).map_err(|e| GhCliError::Archive(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| GhCliError::Archive(e.to_string()))?;
            let name = file.name().replace('\\', "/");
            if name.ends_with("bin/gh.exe")
                || name.ends_with("/gh.exe")
                || name == "gh.exe"
                || name.ends_with("bin/gh")
            {
                let mut out = File::create(&target_exe)
                    .map_err(|e| GhCliError::Io(format!("Failed creating target binary: {e}")))?;
                io::copy(&mut file, &mut out)
                    .map_err(|e| GhCliError::Io(format!("Failed extracting gh binary: {e}")))?;
                info!("Extracted portable gh binary to {:?}", target_exe);
                return Ok(target_exe);
            }
        }

        Err(GhCliError::Archive(
            "gh binary not found within zip archive".into(),
        ))
    }
}

fn find_in_path(exe_name: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for p in env::split_paths(&paths) {
        let full = p.join(exe_name);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}
