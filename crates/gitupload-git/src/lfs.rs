#![forbid(unsafe_code)]
//! Git LFS large binary scanner and `.gitattributes` filter rule writer.

use gitupload_core::errors::GitUploadResult;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Detected large binary file eligible for Git LFS tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LfsCandidate {
    /// Relative path inside the repository.
    pub relative_path: String,
    /// File extension or filename pattern (`*.bin`, `model.onnx`).
    pub pattern: String,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Manages Git LFS detection and `.gitattributes` updates for assets exceeding size thresholds.
pub struct LfsManager;

impl LfsManager {
    /// Scans `repo_root` recursively (skipping `.git`) for files larger than `threshold_mb`.
    pub fn find_large_assets(
        repo_root: &Path,
        threshold_mb: usize,
    ) -> GitUploadResult<Vec<LfsCandidate>> {
        let threshold_bytes = (threshold_mb as u64).saturating_mul(1024 * 1024);
        let mut candidates = Vec::new();
        scan_dir_recursive(repo_root, repo_root, threshold_bytes, &mut candidates)?;
        Ok(candidates)
    }

    /// Ensures each LFS pattern is tracked in `<repo_root>/.gitattributes`.
    pub fn track_patterns(repo_root: &Path, patterns: &[String]) -> GitUploadResult<PathBuf> {
        let attrs_path = repo_root.join(".gitattributes");
        let existing = if attrs_path.is_file() {
            fs::read_to_string(&attrs_path)?
        } else {
            String::new()
        };

        let mut to_append = Vec::new();
        for pat in patterns {
            let trimmed = pat.trim();
            if trimmed.is_empty() {
                continue;
            }
            let rule = format!("{trimmed} filter=lfs diff=lfs merge=lfs -text");
            if !existing.lines().any(|line| line.trim() == rule) && !to_append.contains(&rule) {
                to_append.push(rule);
            }
        }

        if !to_append.is_empty() {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&attrs_path)?;
            if !existing.is_empty() && !existing.ends_with('\n') {
                writeln!(file)?;
            }
            for rule in to_append {
                writeln!(file, "{rule}")?;
            }
        }

        Ok(attrs_path)
    }

    /// Scans `repo_root` for files exceeding `threshold_mb` and auto-registers their patterns in `.gitattributes`.
    pub fn ensure_large_files_tracked(
        repo_root: &Path,
        threshold_mb: usize,
    ) -> GitUploadResult<Vec<LfsCandidate>> {
        let candidates = Self::find_large_assets(repo_root, threshold_mb)?;
        if !candidates.is_empty() {
            let patterns: Vec<String> = candidates.iter().map(|c| c.pattern.clone()).collect();
            Self::track_patterns(repo_root, &patterns)?;
        }
        Ok(candidates)
    }
}

fn scan_dir_recursive(
    root: &Path,
    current: &Path,
    threshold_bytes: u64,
    out: &mut Vec<LfsCandidate>,
) -> GitUploadResult<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        if path.is_dir() {
            scan_dir_recursive(root, &path, threshold_bytes, out)?;
        } else if path.is_file() {
            let size = entry.metadata()?.len();
            if size >= threshold_bytes {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let pattern = match path.extension().and_then(|e| e.to_str()) {
                    Some(ext) if !ext.is_empty() => format!("*.{ext}"),
                    _ => rel.clone(),
                };
                out.push(LfsCandidate {
                    relative_path: rel,
                    pattern,
                    size_bytes: size,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lfs_track() {
        let dir = tempdir().unwrap();
        let big = dir.path().join("model.onnx");
        // Write 2 MiB and test with 1 MiB threshold (simulating >50MB check cleanly in CI)
        fs::write(&big, vec![0u8; 2 * 1024 * 1024]).unwrap();

        let flagged = LfsManager::ensure_large_files_tracked(dir.path(), 1).unwrap();
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].pattern, "*.onnx");

        let attrs = fs::read_to_string(dir.path().join(".gitattributes")).unwrap();
        assert!(attrs.contains("*.onnx filter=lfs diff=lfs merge=lfs -text"));
    }
}
