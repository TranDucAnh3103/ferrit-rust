#![forbid(unsafe_code)]
//! Energy-aware Tokio cron daemon executing periodic silent Git fetches and backups.

use crate::power::PowerMonitor;
use gitupload_core::errors::AppResult;
use gitupload_core::models::SyncScheduleConfig;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// Spawns and manages a cancellable, battery-aware autonomous background synchronization loop.
pub struct ScheduledSyncDaemon;

impl ScheduledSyncDaemon {
    /// Launches the background daemon task polling every `config.interval_minutes` until `cancel_token` fires.
    pub fn spawn_daemon<F, Fut>(
        config: SyncScheduleConfig,
        cancel_token: CancellationToken,
        sync_task: F,
    ) where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AppResult<()>> + Send + 'static,
    {
        let mins = config.interval_minutes.max(1);
        tokio::spawn(async move {
            info!(
                "ScheduledSyncDaemon active (interval={}m, allow_battery={})",
                mins, config.allow_on_battery
            );

            while !cancel_token.is_cancelled() {
                let interval = Duration::from_secs(mins * 60);
                tokio::select! {
                    _ = sleep(interval) => {
                        if !config.enabled {
                            continue;
                        }
                        if !PowerMonitor::should_allow_background_sync(config.allow_on_battery) {
                            info!("Background sync paused cleanly due to OS battery/power policy");
                            continue;
                        }
                        if let Err(err) = sync_task().await {
                            warn!("Scheduled background sync cycle error: {err}");
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        info!("ScheduledSyncDaemon shut down gracefully");
                        break;
                    }
                }
            }
        });
    }
}
