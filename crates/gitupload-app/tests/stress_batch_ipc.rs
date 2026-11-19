use gitupload_core::errors::{AppError, GitUploadError};
use gitupload_core::BatchScheduler;
use gitupload_hooks::{HookInstaller, HookRunner, HooksConfig};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_batch_scheduler_caps_concurrency_at_3_across_20_repos() {
    let scheduler = BatchScheduler::new(3);
    let active = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let items: Vec<usize> = (0..20).collect();
    let active_clone = Arc::clone(&active);
    let peak_clone = Arc::clone(&peak);
    let completed_clone = Arc::clone(&completed);

    let results = scheduler
        .execute_batch(items, move |_idx, cancel| {
            let active_in = Arc::clone(&active_clone);
            let peak_in = Arc::clone(&peak_clone);
            let completed_in = Arc::clone(&completed_clone);
            async move {
                if cancel.is_cancelled() {
                    return Err(AppError::OperationCancelled("cancelled".into()));
                }
                let cur = active_in.fetch_add(1, Ordering::SeqCst) + 1;
                peak_in.fetch_max(cur, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(15)).await;
                active_in.fetch_sub(1, Ordering::SeqCst);
                completed_in.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        })
        .await;

    assert_eq!(results.len(), 20);
    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(completed.load(Ordering::SeqCst), 20);
    assert!(
        peak.load(Ordering::SeqCst) <= 3,
        "Expected peak concurrency <= 3, observed {}",
        peak.load(Ordering::SeqCst)
    );
}

#[tokio::test]
async fn test_batch_scheduler_cancellation_and_hooks_integration() {
    let scheduler = BatchScheduler::new(3);
    let cancel_trigger = scheduler.clone();

    let items: Vec<usize> = (0..12).collect();
    let results = scheduler
        .execute_batch(items, move |idx, token| {
            let ct = cancel_trigger.clone();
            async move {
                if idx == 1 {
                    ct.cancel_all();
                }
                tokio::time::sleep(Duration::from_millis(30)).await;
                if token.is_cancelled() {
                    return Err(AppError::OperationCancelled("aborted".into()));
                }
                Ok(())
            }
        })
        .await;

    assert!(results
        .iter()
        .any(|r| matches!(r, Err(AppError::OperationCancelled(_)))));

    // Cross-crate verification: Hooks installer & runner block secrets before push
    let repo_dir = tempdir().unwrap();
    fs::create_dir_all(repo_dir.path().join(".git/hooks")).unwrap();
    let cfg = HooksConfig::default();
    HookInstaller::install_into_repo(repo_dir.path(), &cfg).unwrap();

    fs::write(
        repo_dir.path().join(".env"),
        "SECRET=ghp_1234567890abcdef1234567890abcdef12",
    )
    .unwrap();
    let err = HookRunner::scan_pre_commit(
        repo_dir.path(),
        &[repo_dir.path().join(".env")],
        &cfg.pre_commit,
    )
    .unwrap_err();
    assert!(matches!(err, GitUploadError::Hook(_)));
}
