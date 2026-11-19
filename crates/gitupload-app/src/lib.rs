#![deny(unsafe_code)]
//! GitUpload Desktop v2.2 Tauri v2 library surface: managed state, micro-batched IPC events, and command handlers.

pub mod commands;
pub mod events;
pub mod marketplace;
pub mod platform;
pub mod state;
pub mod updater_channel;
pub mod webhooks;

pub use state::AppState;
