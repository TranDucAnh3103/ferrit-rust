use chrono::Utc;
use directories::ProjectDirs;
use gitupload_core::marketplace::{MarketplaceIndex, MarketplacePluginEntry, PluginCategory};
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{info, warn};

pub const DEFAULT_MARKETPLACE_URL: &str = "https://marketplace.gitupload.org/v1/plugins.json";

pub struct MarketplaceCatalog {
    client: Client,
    cache_path: PathBuf,
}

impl MarketplaceCatalog {
    pub fn new() -> Self {
        let cache_dir = ProjectDirs::from("com", "stephen", "GitUpload")
            .map(|pd| pd.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".cache"));
        let _ = fs::create_dir_all(&cache_dir);
        let cache_path = cache_dir.join("marketplace.json");

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self { client, cache_path }
    }

    pub fn with_cache_path(cache_path: PathBuf) -> Self {
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { client, cache_path }
    }

    pub fn cache_path(&self) -> &std::path::Path {
        &self.cache_path
    }

    /// Fetches marketplace catalog from remote HTTPS registry with fallback to local offline cache.
    pub async fn fetch_catalog(&self, catalog_url: Option<&str>) -> (MarketplaceIndex, bool) {
        let url = catalog_url.unwrap_or(DEFAULT_MARKETPLACE_URL);

        // Try network fetch first
        match self.client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(index) = resp.json::<MarketplaceIndex>().await {
                    info!(
                        "Successfully fetched remote marketplace index with {} plugins",
                        index.plugins.len()
                    );
                    // Cache to disk
                    if let Ok(json_str) = serde_json::to_string_pretty(&index) {
                        let _ = fs::write(&self.cache_path, json_str);
                    }
                    return (index, false); // is_offline = false
                }
            }
            Ok(resp) => {
                warn!("Marketplace catalog returned HTTP status {}", resp.status());
            }
            Err(e) => {
                warn!("Network error fetching marketplace catalog: {e}");
            }
        }

        // Fallback to offline disk cache
        if self.cache_path.exists() {
            if let Ok(content) = fs::read_to_string(&self.cache_path) {
                if let Ok(cached_index) = serde_json::from_str::<MarketplaceIndex>(&content) {
                    info!(
                        "Loaded marketplace catalog from offline disk cache: {} plugins",
                        cached_index.plugins.len()
                    );
                    return (cached_index, true); // is_offline = true
                }
            }
        }

        // Return default built-in certified catalog if no cache exists
        (Self::default_builtin_catalog(), true)
    }

    /// Default built-in verified plugins index.
    pub fn default_builtin_catalog() -> MarketplaceIndex {
        MarketplaceIndex {
            version: 1,
            updated_at: Utc::now(),
            plugins: vec![
                MarketplacePluginEntry {
                    id: "org.gitupload.plugins.jira-linter".into(),
                    name: "Jira Ticket Commit Linter".into(),
                    version: "1.0.4".into(),
                    author: "GitUpload Core Team".into(),
                    description: "Enforces Jira issue ticket prefix (e.g. PROJ-1234) on all Git commit messages.".into(),
                    category: PluginCategory::Linter,
                    download_url: "https://marketplace.gitupload.org/packages/jira-linter-1.0.4.tar.gz".into(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
                    ed25519_signature: "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
                    publisher_pubkey: "0000000000000000000000000000000000000000000000000000000000000000".into(),
                    min_app_version: "2.6.0".into(),
                    capabilities: vec!["git_read".into(), "commit_hook".into()],
                    verified: true,
                    downloads_count: 1420,
                },
                MarketplacePluginEntry {
                    id: "org.gitupload.plugins.ai-sanitizer".into(),
                    name: "AI Prompt Security Sanitizer".into(),
                    version: "1.1.0".into(),
                    author: "DevSecOps Guild".into(),
                    description: "Redacts PII, private IP ranges, and internal hostnames before dispatching diffs to Cloud AI.".into(),
                    category: PluginCategory::AiPrompt,
                    download_url: "https://marketplace.gitupload.org/packages/ai-sanitizer-1.1.0.tar.gz".into(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
                    ed25519_signature: "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
                    publisher_pubkey: "0000000000000000000000000000000000000000000000000000000000000000".into(),
                    min_app_version: "2.6.0".into(),
                    capabilities: vec!["git_read".into(), "ai_prompt_filter".into()],
                    verified: true,
                    downloads_count: 890,
                },
                MarketplacePluginEntry {
                    id: "org.gitupload.plugins.branch-policy".into(),
                    name: "Branch Naming & Protected Ref Guard".into(),
                    version: "2.0.1".into(),
                    author: "Enterprise Platform Team".into(),
                    description: "Ensures branch names follow conventional prefixes (feat/, fix/, chore/) and blocks direct pushes to protected refs.".into(),
                    category: PluginCategory::Automation,
                    download_url: "https://marketplace.gitupload.org/packages/branch-policy-2.0.1.tar.gz".into(),
                    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
                    ed25519_signature: "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
                    publisher_pubkey: "0000000000000000000000000000000000000000000000000000000000000000".into(),
                    min_app_version: "2.6.0".into(),
                    capabilities: vec!["git_read".into(), "commit_hook".into()],
                    verified: true,
                    downloads_count: 3200,
                },
            ],
        }
    }
}

impl Default for MarketplaceCatalog {
    fn default() -> Self {
        Self::new()
    }
}
