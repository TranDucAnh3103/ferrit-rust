//! Bounded semaphore concurrency scheduler for multi-repository batch operations.

use crate::errors::{AppError, AppResult};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

/// Coordinates parallel asynchronous tasks with bounded concurrency and cooperative cancellation.
#[derive(Clone, Debug)]
pub struct BatchScheduler {
    semaphore: Arc<Semaphore>,
    cancel_token: CancellationToken,
}

impl BatchScheduler {
    /// Creates a new scheduler capping active parallel tasks at `max_concurrent_jobs`.
    pub fn new(max_concurrent_jobs: usize) -> Self {
        let bounded = max_concurrent_jobs.max(1);
        Self {
            semaphore: Arc::new(Semaphore::new(bounded)),
            cancel_token: CancellationToken::new(),
        }
    }

    /// Cancels all queued and running batch operations cooperatively.
    pub fn cancel_all(&self) {
        self.cancel_token.cancel();
    }

    /// Returns a child cancellation token linked to this scheduler.
    pub fn child_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }

    /// Executes `worker_fn` across `items`, guaranteeing no more than `max_concurrent_jobs` run concurrently.
    pub async fn execute_batch<T, F, Fut>(&self, items: Vec<T>, worker_fn: F) -> Vec<AppResult<()>>
    where
        T: Send + 'static,
        F: Fn(T, CancellationToken) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = AppResult<()>> + Send + 'static,
    {
        let worker_fn = Arc::new(worker_fn);
        let mut tasks = Vec::with_capacity(items.len());

        for item in items {
            let sem = Arc::clone(&self.semaphore);
            let token = self.cancel_token.child_token();
            let worker = Arc::clone(&worker_fn);

            tasks.push(tokio::spawn(async move {
                if token.is_cancelled() {
                    return Err(AppError::OperationCancelled(
                        "Batch cancelled before start".into(),
                    ));
                }

                let permit = tokio::select! {
                    _ = token.cancelled() => {
                        return Err(AppError::OperationCancelled(
                            "Batch cancelled while waiting for worker slot".into(),
                        ));
                    }
                    acquired = sem.acquire_owned() => {
                        acquired.map_err(|e| AppError::Internal(e.to_string()))?
                    }
                };

                if token.is_cancelled() {
                    drop(permit);
                    return Err(AppError::OperationCancelled(
                        "Batch cancelled before execution".into(),
                    ));
                }

                let res = worker(item, token).await;
                drop(permit);
                res
            }));
        }

        let mut results = Vec::with_capacity(tasks.len());
        for handle in tasks {
            match handle.await {
                Ok(res) => results.push(res),
                Err(join_err) => results.push(Err(AppError::Internal(join_err.to_string()))),
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn test_scheduler_bounded_and_cancellation() {
        let scheduler = BatchScheduler::new(3);
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let items: Vec<usize> = (0..10).collect();
        let active_cl = Arc::clone(&active);
        let peak_cl = Arc::clone(&peak);

        let results = scheduler
            .execute_batch(items, move |_idx, _tok| {
                let active = Arc::clone(&active_cl);
                let peak = Arc::clone(&peak_cl);
                async move {
                    let cur = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(cur, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;

        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.is_ok()));
        assert!(peak.load(Ordering::SeqCst) <= 3);

        // Verify cancellation latency < 50ms
        let cancel_sched = BatchScheduler::new(2);
        let cancel_handle = cancel_sched.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(15)).await;
            cancel_handle.cancel_all();
        });

        let start = Instant::now();
        let cancelled_res = cancel_sched
            .execute_batch(vec![1, 2, 3, 4], |_item, tok| async move {
                tokio::select! {
                    _ = tok.cancelled() => Err(AppError::OperationCancelled("aborted".into())),
                    _ = tokio::time::sleep(Duration::from_secs(5)) => Ok(()),
                }
            })
            .await;
        assert!(start.elapsed() < Duration::from_millis(250));
        assert!(cancelled_res.iter().all(|r| r.is_err()));
    }
}
