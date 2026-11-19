#![deny(unsafe_code)]
//! Filesystem watching, debounced auto-sync queue, OS power monitoring, and scheduled sync daemon.

pub mod monitor;
pub mod power;
pub mod scheduler;

pub use monitor::{RepoWatcher, WatcherAlert};
pub use power::PowerMonitor;
pub use scheduler::ScheduledSyncDaemon;
