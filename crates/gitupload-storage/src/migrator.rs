//! Legacy Go/Wails (`config.json`, `history.json`, `.env`) one-click zero-loss auto-migrator.

use crate::db::StoragePool;
use crate::repositories::{HistoryRepository, SettingsRepository};
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{AppSettings, PushHistoryRecord};
use gitupload_security::KeyringManager;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Default)]
struct LegacyConfigJson {
    #[serde(default)]
    github_token: String,
    #[serde(default)]
    github_username: String,
    #[serde(default)]
    github_org_name: String,
    #[serde(default)]
    naming_pattern: String,
    #[serde(default)]
    theme_mode: String,
    #[serde(default)]
    git_commit_name: String,
    #[serde(default)]
    git_commit_email: String,
    #[serde(default)]
    repo_private: bool,
    #[serde(default)]
    auto_push: bool,
    #[serde(default)]
    default_commit_msg: String,
    #[serde(default)]
    file_commit_msg_pattern: String,
    #[serde(default)]
    sync_file_repo_name_with_commit: bool,
    #[serde(default)]
    ai_api_key: String,
    #[serde(default)]
    ai_model: String,
}

#[derive(Debug, Deserialize)]
struct LegacyPushedRepo {
    repo_name: String,
    repo_url: String,
    pushed_at: String,
    local_path: String,
}

#[derive(Debug, Deserialize)]
struct LegacyHistoryJson {
    last_pushed: Option<LegacyPushedRepo>,
}

/// Summary of legacy artifacts migrated into Windows Keyring and SQLite.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReport {
    /// True if `config.json` was migrated and sanitized/backed up.
    pub migrated_config_json: bool,
    /// True if `history.json` was migrated into `push_history`.
    pub migrated_history_json: bool,
    /// True if `.env` secrets were migrated and wiped.
    pub migrated_env_file: bool,
    /// Number of secret tokens moved into the OS Credential Vault.
    pub tokens_moved_to_vault: usize,
    /// Paths renamed to `*.migrated_backup`.
    pub backup_files: Vec<PathBuf>,
}

/// Automatic importer for legacy Go/Wails (`%APPDATA%/GGitUpload/`) configuration and `.env` secrets.
pub struct LegacyImporter;

impl LegacyImporter {
    /// Checks standard legacy locations (`%APPDATA%/GGitUpload/config.json`, `history.json`, and local `.env`)
    /// and migrates them safely into OS Keyring + SQLite.
    pub fn migrate_if_needed(
        storage: &StoragePool,
        keyring: &KeyringManager,
    ) -> GitUploadResult<MigrationReport> {
        let app_data_legacy_dir = env::var_os("APPDATA")
            .or_else(|| env::var_os("USERPROFILE"))
            .map(|base| PathBuf::from(base).join("GGitUpload"));

        let config_path = app_data_legacy_dir.as_ref().map(|d| d.join("config.json"));
        let history_path = app_data_legacy_dir.as_ref().map(|d| d.join("history.json"));
        let env_path = Some(PathBuf::from(".env"));

        Self::migrate_from_paths(
            storage,
            keyring,
            config_path.as_deref(),
            history_path.as_deref(),
            env_path.as_deref(),
        )
    }

    /// Migrates explicit legacy paths, stripping plaintext credentials and renaming files to `.migrated_backup`.
    pub fn migrate_from_paths(
        storage: &StoragePool,
        keyring: &KeyringManager,
        config_path: Option<&Path>,
        history_path: Option<&Path>,
        env_path: Option<&Path>,
    ) -> GitUploadResult<MigrationReport> {
        let mut report = MigrationReport::default();
        let settings_repo = SettingsRepository::new(storage);
        let mut settings = settings_repo.load_app_settings()?;
        let mut settings_dirty = false;

        // 1. Migrate legacy config.json
        if let Some(cfg_path) = config_path {
            if cfg_path.is_file() {
                let raw = fs::read_to_string(cfg_path)?;
                let mut legacy: LegacyConfigJson = serde_json::from_str(&raw).map_err(|e| {
                    GitUploadError::Storage(format!("Failed parsing legacy config.json: {e}"))
                })?;

                if !legacy.github_token.trim().is_empty() {
                    let key = KeyringManager::github_account_key("default");
                    keyring.store_token(&key, legacy.github_token.trim())?;
                    report.tokens_moved_to_vault += 1;
                }
                if !legacy.ai_api_key.trim().is_empty() {
                    let provider = infer_provider_from_model(&legacy.ai_model);
                    let key = KeyringManager::ai_provider_key(provider);
                    keyring.store_token(&key, legacy.ai_api_key.trim())?;
                    report.tokens_moved_to_vault += 1;
                }

                apply_legacy_config_fields(&mut settings, &legacy);
                settings_dirty = true;

                // Wipe plaintext secrets before backing up file
                legacy.github_token = "[MIGRATED_TO_OS_KEYRING]".into();
                legacy.ai_api_key = "[MIGRATED_TO_OS_KEYRING]".into();
                let sanitized_json = serde_json::to_string_pretty(&legacy)?;
                let backup_path = append_backup_suffix(cfg_path);
                fs::write(&backup_path, sanitized_json)?;
                fs::remove_file(cfg_path)?;

                report.migrated_config_json = true;
                report.backup_files.push(backup_path);
                info!("Migrated legacy config.json with zero plaintext secrets remaining");
            }
        }

        // 2. Migrate legacy .env
        if let Some(dot_env) = env_path {
            if dot_env.is_file() {
                let raw = fs::read_to_string(dot_env)?;
                let mut sanitized_lines = Vec::new();
                for line in raw.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') {
                        sanitized_lines.push(line.to_string());
                        continue;
                    }
                    if let Some((k, v_raw)) = trimmed.split_once('=') {
                        let key_name = k.trim();
                        let val = unquote_env_value(v_raw.trim());
                        match key_name {
                            "GITHUB_TOKEN" | "GITHUB_TOKEN_FULL_ACCESS" => {
                                if !val.is_empty() {
                                    let vault_key = KeyringManager::github_account_key("default");
                                    keyring.store_token(&vault_key, val)?;
                                    report.tokens_moved_to_vault += 1;
                                }
                                sanitized_lines
                                    .push(format!("{key_name}=[MIGRATED_TO_OS_KEYRING]"));
                            }
                            "AI_API_KEY" => {
                                if !val.is_empty() {
                                    let vault_key = KeyringManager::ai_provider_key("default");
                                    keyring.store_token(&vault_key, val)?;
                                    report.tokens_moved_to_vault += 1;
                                }
                                sanitized_lines
                                    .push(format!("{key_name}=[MIGRATED_TO_OS_KEYRING]"));
                            }
                            "GITHUB_USERNAME" => {
                                if settings.github_username.is_empty() && !val.is_empty() {
                                    settings.github_username = val.to_string();
                                    settings_dirty = true;
                                }
                                sanitized_lines.push(line.to_string());
                            }
                            "GITHUB_ORG_NAME" => {
                                if settings.github_org_name.is_empty() && !val.is_empty() {
                                    settings.github_org_name = val.to_string();
                                    settings_dirty = true;
                                }
                                sanitized_lines.push(line.to_string());
                            }
                            "NAMING_PATTERN" => {
                                if !val.is_empty() {
                                    settings.naming_pattern = val.to_string();
                                    settings_dirty = true;
                                }
                                sanitized_lines.push(line.to_string());
                            }
                            "AI_MODEL" => {
                                if !val.is_empty() {
                                    settings.cloud_ai_model = val.to_string();
                                    settings_dirty = true;
                                }
                                sanitized_lines.push(line.to_string());
                            }
                            _ => sanitized_lines.push(line.to_string()),
                        }
                    } else {
                        sanitized_lines.push(line.to_string());
                    }
                }

                let backup_path = append_backup_suffix(dot_env);
                fs::write(&backup_path, sanitized_lines.join("\n"))?;
                fs::remove_file(dot_env)?;

                report.migrated_env_file = true;
                report.backup_files.push(backup_path);
                info!("Migrated legacy .env secrets into OS Keyring");
            }
        }

        if settings_dirty {
            settings_repo.save_app_settings(&settings)?;
        }

        // 3. Migrate legacy history.json
        if let Some(hist_path) = history_path {
            if hist_path.is_file() {
                let raw = fs::read_to_string(hist_path)?;
                let legacy_hist: LegacyHistoryJson = serde_json::from_str(&raw).map_err(|e| {
                    GitUploadError::Storage(format!("Failed parsing legacy history.json: {e}"))
                })?;

                if let Some(last) = legacy_hist.last_pushed {
                    let parsed_time = DateTime::parse_from_rfc3339(&last.pushed_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let record = PushHistoryRecord {
                        id: Uuid::new_v4().to_string(),
                        account_id: "default".into(),
                        repo_name: last.repo_name,
                        repo_url: gitupload_security::sanitize_log_message(&last.repo_url),
                        local_path: last.local_path,
                        commit_sha: None,
                        commit_msg: "migrated-legacy-push".into(),
                        files_count: 1,
                        status: "SUCCESS".into(),
                        error_message: None,
                        is_force_with_lease: false,
                        duration_ms: 0,
                        created_at: parsed_time,
                    };
                    HistoryRepository::new(storage).save(&record)?;
                }

                let backup_path = append_backup_suffix(hist_path);
                let sanitized_raw = gitupload_security::sanitize_log_message(&raw);
                fs::write(&backup_path, sanitized_raw)?;
                fs::remove_file(hist_path)?;
                report.migrated_history_json = true;
                report.backup_files.push(backup_path);
            }
        }

        Ok(report)
    }
}

fn apply_legacy_config_fields(target: &mut AppSettings, src: &LegacyConfigJson) {
    if !src.github_username.trim().is_empty() {
        target.github_username = src.github_username.trim().to_string();
    }
    if !src.github_org_name.trim().is_empty() {
        target.github_org_name = src.github_org_name.trim().to_string();
    }
    if !src.naming_pattern.trim().is_empty() {
        target.naming_pattern = src.naming_pattern.trim().to_string();
    }
    if !src.theme_mode.trim().is_empty() {
        target.theme_mode = src.theme_mode.trim().to_string();
    }
    if !src.git_commit_name.trim().is_empty() {
        target.git_commit_name = src.git_commit_name.trim().to_string();
    }
    if !src.git_commit_email.trim().is_empty() {
        target.git_commit_email = src.git_commit_email.trim().to_string();
    }
    target.repo_private = src.repo_private;
    target.auto_push = src.auto_push;
    if !src.default_commit_msg.trim().is_empty() {
        target.default_commit_msg = src.default_commit_msg.trim().to_string();
    }
    if !src.file_commit_msg_pattern.trim().is_empty() {
        target.file_commit_msg_pattern = src.file_commit_msg_pattern.trim().to_string();
    }
    target.sync_file_repo_name_with_commit = src.sync_file_repo_name_with_commit;
    if !src.ai_model.trim().is_empty() {
        target.cloud_ai_model = src.ai_model.trim().to_string();
        target.cloud_ai_provider = infer_provider_from_model(&src.ai_model).to_string();
    }
}

fn infer_provider_from_model(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("gemini") {
        "gemini"
    } else if lower.contains("claude") {
        "claude"
    } else if lower.contains("gpt") {
        "openai"
    } else if lower.contains("llama") || lower.contains("mixtral") {
        "groq"
    } else if lower.is_empty() {
        "default"
    } else {
        "openrouter"
    }
}

fn unquote_env_value(val: &str) -> &str {
    if val.len() >= 2
        && ((val.starts_with('"') && val.ends_with('"'))
            || (val.starts_with('\'') && val.ends_with('\'')))
    {
        &val[1..val.len() - 1]
    } else {
        val
    }
}

fn append_backup_suffix(path: &Path) -> PathBuf {
    let mut os_str = path.as_os_str().to_os_string();
    os_str.push(".migrated_backup");
    PathBuf::from(os_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use tempfile::tempdir;

    #[test]
    fn test_migration() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("migrated.db");
        let storage = StoragePool::new(db_path.clone()).unwrap();
        let keyring = KeyringManager::new_in_memory();

        let fake_pat = "ghp_1234567890abcdef1234567890abcdef12";
        let fake_ai_key = "sk-testkey1234567890abcdefghij";

        let config_path = dir.path().join("config.json");
        let legacy_cfg = format!(
            r#"{{
                "github_token": "{fake_pat}",
                "github_username": "stephen",
                "github_org_name": "rikkei-class",
                "naming_pattern": "session%02d-ex%02d-homework",
                "theme_mode": "dark",
                "repo_private": true,
                "ai_api_key": "{fake_ai_key}",
                "ai_model": "gemini-2.0-flash"
            }}"#
        );
        fs::write(&config_path, legacy_cfg).unwrap();

        let history_path = dir.path().join("history.json");
        let legacy_hist = r#"{
            "last_pushed": {
                "repo_name": "session15-ex02-homework",
                "repo_url": "https://github.com/rikkei-class/session15-ex02-homework",
                "pushed_at": "2026-11-18T10:00:00Z",
                "local_path": "F:/Repos/SS15/EX02"
            }
        }"#;
        fs::write(&history_path, legacy_hist).unwrap();

        let env_path = dir.path().join(".env");
        fs::write(&env_path, "GITHUB_USERNAME=\"stephen\"\n").unwrap();

        let report = LegacyImporter::migrate_from_paths(
            &storage,
            &keyring,
            Some(&config_path),
            Some(&history_path),
            Some(&env_path),
        )
        .unwrap();

        assert!(report.migrated_config_json);
        assert!(report.migrated_history_json);
        assert!(report.migrated_env_file);
        assert_eq!(report.tokens_moved_to_vault, 2);

        // Original files removed and replaced by .migrated_backup
        assert!(!config_path.exists());
        assert!(!history_path.exists());
        assert!(!env_path.exists());

        let backup_cfg =
            fs::read_to_string(dir.path().join("config.json.migrated_backup")).unwrap();
        assert!(!backup_cfg.contains(fake_pat));
        assert!(!backup_cfg.contains(fake_ai_key));

        // Tokens exist in Keyring
        let stored_gh = keyring
            .get_token(&KeyringManager::github_account_key("default"))
            .unwrap()
            .unwrap();
        assert_eq!(stored_gh.expose_secret(), fake_pat);

        let stored_ai = keyring
            .get_token(&KeyringManager::ai_provider_key("gemini"))
            .unwrap()
            .unwrap();
        assert_eq!(stored_ai.expose_secret(), fake_ai_key);

        // Settings and history exist in SQLite without plaintext secrets
        let settings = SettingsRepository::new(&storage)
            .load_app_settings()
            .unwrap();
        assert_eq!(settings.github_username, "stephen");
        assert_eq!(settings.github_org_name, "rikkei-class");
        assert!(settings.repo_private);

        let last = HistoryRepository::new(&storage)
            .get_last_pushed()
            .unwrap()
            .unwrap();
        assert_eq!(last.repo_name, "session15-ex02-homework");

        let raw_db_bytes = fs::read(&db_path).unwrap();
        let raw_db_lossy = String::from_utf8_lossy(&raw_db_bytes);
        assert!(!raw_db_lossy.contains(fake_pat));
        assert!(!raw_db_lossy.contains(fake_ai_key));
    }
}
