//! Autonomous background synchronization schedule, power policy, and outcome models.

use serde::{Deserialize, Serialize};

/// Battery & AC power state observed from the operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    /// Plugged into wall AC power.
    AcConnected,
    /// Discharging on healthy battery level (> 20%, Battery Saver off).
    BatteryNormal,
    /// Low battery (`<= 20%`) or Battery Saver mode active.
    BatteryLowOrSaver,
}

/// Configuration for the autonomous background fetch, fast-forward, and bundle backup daemon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncScheduleConfig {
    /// Whether scheduled background sync cycles are enabled.
    pub enabled: bool,
    /// Polling interval in minutes (minimum 1).
    pub interval_minutes: u64,
    /// Permit background sync on normal battery power (still halts on low/battery-saver).
    pub allow_on_battery: bool,
    /// Periodically generate standalone `.bundle` backups in app data directory.
    pub backup_bundles_enabled: bool,
}

impl Default for SyncScheduleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 30,
            allow_on_battery: false,
            backup_bundles_enabled: true,
        }
    }
}

/// Outcome of a single repository autonomous background sync check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoSyncOutcome {
    /// Local branch already matches remote tracking branch.
    UpToDate,
    /// Clean worktree safely fast-forwarded `commits_pulled` commits.
    FastForwarded { commits_pulled: usize },
    /// Local and remote diverged or dirty worktree prevented fast-forward.
    Diverged { ahead: usize, behind: usize },
}
