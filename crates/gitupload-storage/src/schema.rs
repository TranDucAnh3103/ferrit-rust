//! SQLite schema DDL for GitUpload Desktop v2.2.

/// Initial schema DDL establishing settings, push audit history, orgs, PRs, hooks, and tracked repos.
pub const INITIAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    username TEXT NOT NULL,
    avatar_url TEXT,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS organizations (
    login TEXT PRIMARY KEY NOT NULL,
    id INTEGER NOT NULL,
    avatar_url TEXT NOT NULL,
    description TEXT,
    is_admin BOOLEAN NOT NULL DEFAULT 0,
    role TEXT NOT NULL DEFAULT 'member',
    can_create_repo BOOLEAN NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tracked_repositories (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    org_login TEXT,
    repo_name TEXT NOT NULL,
    local_path TEXT NOT NULL,
    remote_url TEXT NOT NULL,
    default_branch TEXT NOT NULL DEFAULT 'main',
    has_lfs BOOLEAN NOT NULL DEFAULT 0,
    has_hooks BOOLEAN NOT NULL DEFAULT 0,
    signing_enabled BOOLEAN NOT NULL DEFAULT 0,
    last_synced_at DATETIME,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(account_id, local_path)
);

CREATE TABLE IF NOT EXISTS push_history (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL DEFAULT 'default',
    repo_id TEXT,
    repo_name TEXT NOT NULL,
    repo_url TEXT NOT NULL,
    local_path TEXT NOT NULL,
    commit_sha TEXT,
    commit_msg TEXT NOT NULL,
    files_count INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'SUCCESS',
    error_message TEXT,
    is_force_with_lease BOOLEAN NOT NULL DEFAULT 0,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    pushed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS pull_requests (
    id INTEGER PRIMARY KEY,
    number INTEGER NOT NULL,
    repo_full_name TEXT NOT NULL,
    title TEXT NOT NULL,
    state TEXT NOT NULL,
    html_url TEXT NOT NULL,
    created_at DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS hooks_config (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS github_cli_metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_history_pushed_at ON push_history(pushed_at DESC);
CREATE INDEX IF NOT EXISTS idx_tracked_path ON tracked_repositories(local_path);
"#;

/// Schema v2 migration DDL adding multi-remote mirroring targets and managed SSH key descriptors.
pub const SCHEMA_V2_REMOTES_SSH: &str = r#"
CREATE TABLE IF NOT EXISTS repository_remotes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repo_path TEXT NOT NULL,
    remote_name TEXT NOT NULL,
    remote_url TEXT NOT NULL,
    provider TEXT NOT NULL DEFAULT 'custom',
    sync_policy TEXT NOT NULL DEFAULT 'bidirectional',
    auth_account TEXT NOT NULL DEFAULT '',
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    timeout_seconds INTEGER NOT NULL DEFAULT 30,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(repo_path, remote_name)
);

CREATE TABLE IF NOT EXISTS ssh_keys (
    key_id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fingerprint_sha256 TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL,
    is_registered_on_github BOOLEAN NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_repo_remotes_path ON repository_remotes(repo_path);
"#;

/// Schema v3 migration DDL adding commit velocity analytics and classroom deadline tracking.
pub const SCHEMA_V3_ANALYTICS: &str = r#"
CREATE TABLE IF NOT EXISTS daily_commit_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    commit_count INTEGER NOT NULL DEFAULT 0,
    lines_added INTEGER NOT NULL DEFAULT 0,
    lines_deleted INTEGER NOT NULL DEFAULT 0,
    author_email TEXT NOT NULL DEFAULT '',
    UNIQUE(date, repo_name, author_email)
);

CREATE TABLE IF NOT EXISTS classroom_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_num INTEGER NOT NULL,
    exercise_num INTEGER NOT NULL,
    title TEXT NOT NULL,
    deadline_utc DATETIME NOT NULL,
    UNIQUE(session_num, exercise_num)
);

CREATE INDEX IF NOT EXISTS idx_commit_stats_date ON daily_commit_stats(date DESC);
CREATE INDEX IF NOT EXISTS idx_commit_stats_author ON daily_commit_stats(author_email, date DESC);
"#;

/// Schema v4 migration DDL adding semantic commit vector embeddings BLOBs and DevSecOps scan history.
pub const SCHEMA_V4_EMBEDDINGS_SECURITY: &str = r#"
CREATE TABLE IF NOT EXISTS commit_embeddings (
    repo_path TEXT NOT NULL,
    commit_oid TEXT NOT NULL,
    summary TEXT NOT NULL,
    embedding_blob BLOB NOT NULL,
    dimensions INTEGER NOT NULL,
    indexed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (repo_path, commit_oid)
);

CREATE TABLE IF NOT EXISTS security_scan_history (
    scan_id TEXT PRIMARY KEY NOT NULL,
    repo_path TEXT NOT NULL,
    total_findings INTEGER NOT NULL DEFAULT 0,
    critical_count INTEGER NOT NULL DEFAULT 0,
    high_count INTEGER NOT NULL DEFAULT 0,
    passed BOOLEAN NOT NULL DEFAULT 1,
    scan_duration_ms INTEGER NOT NULL DEFAULT 0,
    scanned_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_embeddings_repo ON commit_embeddings(repo_path);
CREATE INDEX IF NOT EXISTS idx_scan_history_repo ON security_scan_history(repo_path, scanned_at DESC);
"#;

/// Schema v5 migration DDL establishing user WASM plugins and multi-channel webhook dispatcher history.
pub const SCHEMA_V5_PLUGINS_WEBHOOKS: &str = r#"
CREATE TABLE IF NOT EXISTS user_plugins (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    author TEXT NOT NULL,
    entrypoint TEXT NOT NULL,
    capabilities TEXT NOT NULL,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS webhook_endpoints (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    transport TEXT NOT NULL CHECK(transport IN ('discord', 'slack', 'telegram', 'generic')),
    target_url TEXT NOT NULL,
    vault_secret_key TEXT,
    events TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    endpoint_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    status_code INTEGER,
    attempt_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    error_message TEXT,
    delivered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(endpoint_id) REFERENCES webhook_endpoints(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_endpoint ON webhook_deliveries(endpoint_id, delivered_at DESC);
"#;
