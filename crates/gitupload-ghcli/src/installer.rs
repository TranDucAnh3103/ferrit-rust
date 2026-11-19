//! Portable GitHub CLI downloader with GitHub Releases discovery and SHA256 checksum validation.

use crate::lifecycle::{GhLifecycleManager, GH_OFFICIAL_REPO};
use gitupload_core::errors::{GhCliError, GhCliResult};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

/// Downloads and verifies official `cli/cli` portable archives into the user sandbox.
pub struct GhInstaller;

impl GhInstaller {
    /// Verifies `bytes` against `expected_hex` SHA256 digest (case-insensitive).
    pub fn verify_checksum(bytes: &[u8], expected_hex: &str) -> bool {
        GhLifecycleManager::verify_checksum(bytes, expected_hex).is_ok()
    }

    /// Parses a `checksums.txt` manifest and extracts the SHA256 hash matching `asset_name`.
    pub fn parse_checksum_for_asset(checksums_txt: &str, asset_name: &str) -> Option<String> {
        for line in checksums_txt.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1].trim_start_matches('*') == asset_name {
                return Some(parts[0].to_string());
            }
        }
        None
    }

    /// Downloads the latest official Windows amd64 portable zip from `cli/cli`, verifies SHA256,
    /// and extracts `gh.exe` into `dest_dir` without requiring admin rights.
    pub async fn download_and_extract_sandbox(dest_dir: &Path) -> GhCliResult<PathBuf> {
        Self::download_and_extract_from_api("https://api.github.com", GH_OFFICIAL_REPO, dest_dir)
            .await
    }

    /// Parameterized installer allowing mock API base URLs for deterministic offline testing.
    pub async fn download_and_extract_from_api(
        api_base: &str,
        repo_slug: &str,
        dest_dir: &Path,
    ) -> GhCliResult<PathBuf> {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static("GitUpload-Desktop/2.2"),
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| GhCliError::Network(e.to_string()))?;

        let release_url = format!(
            "{}/repos/{}/releases/latest",
            api_base.trim_end_matches('/'),
            repo_slug
        );
        let release: GitHubRelease = client
            .get(&release_url)
            .send()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| GhCliError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?;

        let version_no_v = release.tag_name.trim_start_matches('v');
        let target_asset_name = format!("gh_{version_no_v}_windows_amd64.zip");
        let checksum_asset_name = format!("gh_{version_no_v}_checksums.txt");

        let zip_asset = release
            .assets
            .iter()
            .find(|a| a.name == target_asset_name)
            .ok_or_else(|| {
                GhCliError::Archive(format!("Asset '{target_asset_name}' not found in release"))
            })?;

        let checksum_asset = release
            .assets
            .iter()
            .find(|a| a.name == checksum_asset_name)
            .ok_or_else(|| {
                GhCliError::Archive(format!("Checksum asset '{checksum_asset_name}' not found"))
            })?;

        let checksums_txt = client
            .get(&checksum_asset.browser_download_url)
            .send()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?
            .text()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?;

        let expected_sha = Self::parse_checksum_for_asset(&checksums_txt, &target_asset_name)
            .ok_or_else(|| {
                GhCliError::Archive(format!("SHA256 entry missing for {target_asset_name}"))
            })?;

        let zip_bytes = client
            .get(&zip_asset.browser_download_url)
            .send()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?
            .bytes()
            .await
            .map_err(|e| GhCliError::Network(e.to_string()))?;

        GhLifecycleManager::verify_checksum(&zip_bytes, &expected_sha)?;

        let manager = GhLifecycleManager::with_sandbox_dir(dest_dir.to_path_buf());
        manager.extract_gh_exe(&zip_bytes)
    }
}
