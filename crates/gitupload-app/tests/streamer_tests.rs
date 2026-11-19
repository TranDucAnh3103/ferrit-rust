use gitupload_app::events::{EventStreamer, JobLogBatchPayload};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_micro_batch_event_streamer_ac_p3_04() {
    let (raw_tx, raw_rx) = mpsc::channel::<(String, String)>(2048);
    let (batch_tx, mut batch_rx) = mpsc::unbounded_channel::<JobLogBatchPayload>();

    let streamer_handle = EventStreamer::spawn_log_streamer_with_sink(
        Arc::new(batch_tx),
        "job-ac-p3-04".to_string(),
        raw_rx,
    );

    // Send 1,000 log lines spread across ~40ms (simulating fast git CLI bursts + embedded secrets).
    for i in 0..1000 {
        let msg = if i % 100 == 0 {
            format!(
                "Pushing batch {i} with https://ghp_1234567890abcdef1234567890abcdef1234@github.com/org/repo.git and github_pat_1234567890abcdef123456"
            )
        } else {
            format!("Counting objects: {i}/1000 done")
        };
        raw_tx.send(("info".to_string(), msg)).await.unwrap();
        if i == 500 {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }
    drop(raw_tx);

    streamer_handle.await.unwrap();

    let mut batches = Vec::new();
    while let Ok(batch) = batch_rx.try_recv() {
        batches.push(batch);
    }

    // AC-P3-04: <= 3 batched IPC events for 1,000 log lines in <= 50ms.
    assert!(
        !batches.is_empty() && batches.len() <= 3,
        "Expected 1..=3 batches, got {}",
        batches.len()
    );

    let total_lines: usize = batches.iter().map(|b| b.lines.len()).sum();
    assert_eq!(total_lines, 1000);

    // SEC-02: Verify zero raw tokens survive across every serialized batch.
    for batch in &batches {
        assert_eq!(batch.job_id, "job-ac-p3-04");
        let serialized = serde_json::to_string(batch).unwrap();
        assert!(!serialized.contains("ghp_1234567890"));
        assert!(!serialized.contains("github_pat_1234567890"));
    }
}
