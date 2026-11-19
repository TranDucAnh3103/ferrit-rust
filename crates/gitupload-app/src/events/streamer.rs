//! Throttled 30ms (~33-60Hz) micro-batched event streamer with real-time SEC-02 token sanitization.

use gitupload_security::sanitizer::sanitize_log_message;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Throttled progress update payload emitted on channel `job-progress`.
#[derive(Clone, Debug, Serialize)]
pub struct JobProgressPayload {
    /// Unique job identifier.
    pub job_id: String,
    /// Current pipeline stage label.
    pub stage: String,
    /// Completion percentage (`0..=100`).
    pub percent: u32,
    /// Sanitized human-readable status message.
    pub message: String,
}

/// Micro-batched log payload emitted on channel `job-log-batch`.
#[derive(Clone, Debug, Serialize)]
pub struct JobLogBatchPayload {
    /// Job identifier.
    pub job_id: String,
    /// Batch of token-sanitized log entries.
    pub lines: Vec<SanitizedLogLine>,
}

/// Individual sanitized log line within a batch.
#[derive(Clone, Debug, Serialize)]
pub struct SanitizedLogLine {
    /// Local timestamp (`HH:MM:SS.mmm`).
    pub timestamp: String,
    /// Severity level (`info`, `warn`, `error`, `success`).
    pub level: String,
    /// Redacted log message text.
    pub text: String,
}

/// Abstraction over event emission targets (Tauri `AppHandle` in production, channels in tests).
pub trait LogSink: Send + Sync + 'static {
    /// Emits a sanitized batch of log lines.
    fn emit_batch(&self, payload: JobLogBatchPayload);
}

impl LogSink for AppHandle {
    fn emit_batch(&self, payload: JobLogBatchPayload) {
        let _ = self.emit("job-log-batch", payload);
    }
}

impl LogSink for mpsc::UnboundedSender<JobLogBatchPayload> {
    fn emit_batch(&self, payload: JobLogBatchPayload) {
        let _ = self.send(payload);
    }
}

/// Safety cap per batch to prevent unbounded memory growth on runaway streams.
const MAX_BATCH_LINES: usize = 1024;

/// Micro-batches high-throughput Git/CLI log streams at a 30ms / 1024-line window.
pub struct EventStreamer;

impl EventStreamer {
    /// Spawns a micro-batched log streamer delivering sanitized batches to Tauri `AppHandle`.
    pub fn spawn_log_streamer(
        app_handle: AppHandle,
        job_id: String,
        receiver: mpsc::Receiver<(String, String)>,
    ) -> JoinHandle<()> {
        Self::spawn_log_streamer_with_sink(Arc::new(app_handle), job_id, receiver)
    }

    /// Spawns a micro-batched log streamer targeting an abstract `LogSink` (for production and CI tests).
    pub fn spawn_log_streamer_with_sink<S: LogSink>(
        sink: Arc<S>,
        job_id: String,
        mut receiver: mpsc::Receiver<(String, String)>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut buffer: Vec<SanitizedLogLine> = Vec::with_capacity(64);
            let mut flush_deadline: Option<tokio::time::Instant> = None;

            loop {
                tokio::select! {
                    maybe_msg = receiver.recv() => {
                        match maybe_msg {
                            Some((level, raw_text)) => {
                                let sanitized = sanitize_log_message(&raw_text);
                                let now = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
                                if buffer.is_empty() {
                                    flush_deadline = Some(
                                        tokio::time::Instant::now() + Duration::from_millis(30),
                                    );
                                }
                                buffer.push(SanitizedLogLine {
                                    timestamp: now,
                                    level,
                                    text: sanitized,
                                });

                                if buffer.len() >= MAX_BATCH_LINES {
                                    flush_deadline = None;
                                    let payload = JobLogBatchPayload {
                                        job_id: job_id.clone(),
                                        lines: std::mem::take(&mut buffer),
                                    };
                                    sink.emit_batch(payload);
                                }
                            }
                            None => {
                                if !buffer.is_empty() {
                                    sink.emit_batch(JobLogBatchPayload {
                                        job_id: job_id.clone(),
                                        lines: std::mem::take(&mut buffer),
                                    });
                                }
                                break;
                            }
                        }
                    }
                    _ = async {
                        match flush_deadline {
                            Some(deadline) => tokio::time::sleep_until(deadline).await,
                            None => std::future::pending::<()>().await,
                        }
                    } => {
                        flush_deadline = None;
                        if !buffer.is_empty() {
                            sink.emit_batch(JobLogBatchPayload {
                                job_id: job_id.clone(),
                                lines: std::mem::take(&mut buffer),
                            });
                        }
                    }
                }
            }
        })
    }

    /// Emits a debounced sanitized `job-progress` payload.
    pub fn emit_progress(
        app_handle: &AppHandle,
        job_id: &str,
        stage: &str,
        percent: u32,
        message: &str,
    ) {
        let _ = app_handle.emit(
            "job-progress",
            JobProgressPayload {
                job_id: job_id.to_string(),
                stage: stage.to_string(),
                percent: percent.min(100),
                message: sanitize_log_message(message),
            },
        );
    }
}
