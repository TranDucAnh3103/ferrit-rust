//! Async CLI Git fallback engine isolated inside Windows Job Objects.

pub mod job_object;
pub mod runner;

pub use job_object::WindowsJobObject;
pub use runner::CliGitEngine;
