#![forbid(unsafe_code)]
//! Zero-dependency native Git hooks runner in pure Rust.

pub mod config;
pub mod installer;
pub mod runner;

pub use config::{CommitMsgConfig, HooksConfig, PreCommitConfig, PrePushConfig};
pub use installer::{HookInstallStatus, HookInstaller};
pub use runner::{HookRunner, PreCommitReport, SecretFinding};
