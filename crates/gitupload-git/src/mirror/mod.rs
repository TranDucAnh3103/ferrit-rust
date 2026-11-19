//! Multi-remote parallel synchronization coordinator and pre-flight endpoint health probe.

pub mod coordinator;
pub mod probe;

pub use coordinator::MultiRemoteCoordinator;
pub use probe::RemoteHealthChecker;
