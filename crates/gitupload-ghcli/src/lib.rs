#![forbid(unsafe_code)]
//! GitHub CLI (`gh`) Full Lifecycle Manager & Dual-Auth Bridge.

pub mod auth_bridge;
pub mod installer;
pub mod lifecycle;
pub mod manager;

pub use auth_bridge::GhAuthBridge;
pub use installer::GhInstaller;
pub use lifecycle::GhLifecycleManager;
pub use manager::GhStatusReport;
