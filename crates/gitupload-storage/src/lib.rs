#![forbid(unsafe_code)]
//! SQLite storage engine with WAL mode, relational repositories, and legacy config auto-migration.

pub mod analytics_repo;
pub mod db;
pub mod embeddings;
pub mod migrator;
pub mod plugins_repo;
pub mod remote_ssh_repo;
pub mod repositories;
pub mod schema;
pub mod webhooks_repo;

pub use analytics_repo::{compute_heatmap_tier, AnalyticsRepository};
pub use db::{DbPool, StoragePool};
pub use embeddings::EmbeddingStore;
pub use migrator::{LegacyImporter, MigrationReport};
pub use plugins_repo::PluginsRepository;
pub use remote_ssh_repo::{RemoteRepository, SshKeyRepository};
pub use repositories::{
    HistoryRepository, HooksConfigRepository, OrgRepository, SettingsRepository,
};
pub use webhooks_repo::WebhooksRepository;
