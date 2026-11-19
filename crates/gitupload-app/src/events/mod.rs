#![forbid(unsafe_code)]
//! Micro-batched, token-sanitized Tauri IPC event channels.

pub mod streamer;

pub use streamer::{
    EventStreamer, JobLogBatchPayload, JobProgressPayload, LogSink, SanitizedLogLine,
};
