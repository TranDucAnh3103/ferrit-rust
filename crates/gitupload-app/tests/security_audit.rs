use gitupload_security::{sanitize_log_message, KeyringManager};
use gitupload_storage::{HistoryRepository, LegacyImporter, SettingsRepository, StoragePool};
use secrecy::ExposeSecret;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_sec01_sec02_and_zero_plaintext_migration_audit() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("audit.db");
    let pool = StoragePool::new(db_path.clone()).unwrap();
    let vault = KeyringManager::new_in_memory();

    let fake_pat = "ghp_0123456789abcdef0123456789abcdef0123";
    let fake_ai = "AIzaSyFakeSecretAuditKey1234567890";

    let legacy_cfg = dir.path().join("config.json");
    let legacy_hist = dir.path().join("history.json");
    let legacy_env = dir.path().join(".env");

    fs::write(
        &legacy_cfg,
        serde_json::json!({
            "github_token": fake_pat,
            "github_username": "stephen-audit",
            "naming_pattern": "prefix-{repo}",
            "ai_api_key": fake_ai,
            "ai_model": "gemini-2.0-flash"
        })
        .to_string(),
    )
    .unwrap();

    fs::write(
        &legacy_hist,
        serde_json::json!({
            "last_pushed": {
                "repo_name": "prefix-audit-repo",
                "repo_url": format!("https://{fake_pat}@github.com/stephen-audit/prefix-audit-repo"),
                "pushed_at": "2026-11-19T00:00:00Z",
                "local_path": "C:/projects/audit-repo"
            }
        })
        .to_string(),
    )
    .unwrap();

    fs::write(
        &legacy_env,
        format!("GITHUB_TOKEN={fake_pat}\nAI_API_KEY={fake_ai}\n"),
    )
    .unwrap();

    let report = LegacyImporter::migrate_from_paths(
        &pool,
        &vault,
        Some(&legacy_cfg),
        Some(&legacy_hist),
        Some(&legacy_env),
    )
    .unwrap();

    assert!(report.migrated_config_json);
    assert!(report.migrated_history_json);
    assert!(report.migrated_env_file);

    // 1. Secrets must live only in Vault and never leak via Debug/logs.
    let gh_secret = vault
        .get_token(&KeyringManager::github_account_key("default"))
        .unwrap()
        .unwrap();
    assert_eq!(gh_secret.expose_secret(), fake_pat);
    assert!(!format!("{gh_secret:?}").contains(fake_pat));

    // 2. Non-sensitive settings and sanitized history exist in SQLite WAL without token leakage.
    let settings = SettingsRepository::new(&pool).load_app_settings().unwrap();
    assert_eq!(settings.github_username, "stephen-audit");

    let history = HistoryRepository::new(&pool).list_recent(10).unwrap();
    assert_eq!(history.len(), 1);
    assert!(!history[0].repo_url.contains(fake_pat));

    // 3. Raw SQLite bytes must never contain any migrated token pattern.
    let db_bytes = fs::read(&db_path).unwrap();
    let db_lossy = String::from_utf8_lossy(&db_bytes);
    assert!(!db_lossy.contains(fake_pat));
    assert!(!db_lossy.contains(fake_ai));

    // 4. Backup JSON files must no longer retain plaintext secrets.
    let migrated_cfg = dir.path().join("config.json.migrated_backup");
    if migrated_cfg.exists() {
        let backup_text = fs::read_to_string(migrated_cfg).unwrap();
        assert!(!backup_text.contains(fake_pat));
        assert!(!backup_text.contains(fake_ai));
    }

    // 5. Log sanitizer strips every supported token format.
    let dirty_log = format!(
        "Failed push to https://{fake_pat}@github.com/u/r.git with header Authorization: Bearer github_pat_1234567890abcdef1234567890 and {fake_ai}"
    );
    let clean_log = sanitize_log_message(&dirty_log);
    assert!(!clean_log.contains(fake_pat));
    assert!(!clean_log.contains("github_pat_"));
    assert!(!clean_log.contains(fake_ai));
}
