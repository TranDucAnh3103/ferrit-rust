use gitupload_core::models::{PowerState, SyncScheduleConfig};
use gitupload_watcher::{PowerMonitor, ScheduledSyncDaemon};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[test]
fn test_power_policy_matrix() {
    assert!(PowerMonitor::evaluate_policy(
        PowerState::AcConnected,
        false
    ));
    assert!(!PowerMonitor::evaluate_policy(
        PowerState::BatteryNormal,
        false
    ));
    assert!(PowerMonitor::evaluate_policy(
        PowerState::BatteryNormal,
        true
    ));
    assert!(!PowerMonitor::evaluate_policy(
        PowerState::BatteryLowOrSaver,
        false
    ));
    assert!(!PowerMonitor::evaluate_policy(
        PowerState::BatteryLowOrSaver,
        true
    ));
}

#[tokio::test]
async fn test_daemon_graceful_cancellation_under_50ms() {
    let token = CancellationToken::new();
    let config = SyncScheduleConfig {
        enabled: true,
        interval_minutes: 60,
        allow_on_battery: false,
        backup_bundles_enabled: false,
    };

    let counter = Arc::new(AtomicUsize::new(0));
    let c = Arc::clone(&counter);

    ScheduledSyncDaemon::spawn_daemon(config, token.clone(), move || {
        let c = Arc::clone(&c);
        async move {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });

    token.cancel();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    assert!(token.is_cancelled());
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}
