#![forbid(unsafe_code)]

//! WebAssembly Plugin Sandbox & Runtime Engine for GitUpload.
//!
//! Provides isolated execution of third-party extensions within a 32MB linear memory
//! sandbox, instruction fuel metering, capability enforcement, cryptographic package
//! verification (Ed25519 & SHA256), and atomic installation with rollback safety.

pub mod engine;
pub mod installer;
pub mod linker;
pub mod runner;
pub mod sandbox;
pub mod verify;

pub use engine::PluginEngine;
pub use installer::PluginInstaller;
pub use linker::build_linker;
pub use runner::PluginRunner;
pub use sandbox::{HostState, PluginError};
pub use verify::verify_plugin_bundle;
