use chrono::{Duration, NaiveDate};
use criterion::{criterion_group, criterion_main, Criterion};
use gitupload_core::models::DailyCommitStat;
use gitupload_storage::{AnalyticsRepository, StoragePool};
use std::time::Instant;

fn bench_analytics_50k_rows(c: &mut Criterion) {
    let pool = StoragePool::in_memory().unwrap();
    let analytics = AnalyticsRepository::new(&pool);
    let today = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();

    // Seed 50,000 realistic commit stats across 100 authors and 500 days
    let mut rows = Vec::with_capacity(50_000);
    for a in 0..100 {
        let email = format!("engineer_{a:03}@gitupload.local");
        for d in 0..500 {
            let date = (today - Duration::days(d)).format("%Y-%m-%d").to_string();
            rows.push(DailyCommitStat {
                date,
                repo_name: format!("service_{}", d % 5),
                commit_count: ((a + d) % 9 + 1) as u32,
                lines_added: 45,
                lines_deleted: 12,
                author_email: email.clone(),
            });
        }
    }
    analytics.upsert_daily_stats_batch(&rows).unwrap();

    // One-shot SLA assertion (< 5 ms for author heatmap query)
    let t0 = Instant::now();
    let hm = analytics
        .query_365_day_heatmap(Some("engineer_042@gitupload.local"), today)
        .unwrap();
    assert_eq!(hm.len(), 365);
    assert!(
        t0.elapsed().as_millis() < 20,
        "Cold heatmap query over 50k rows exceeded budget: {:?}",
        t0.elapsed()
    );

    c.bench_function("analytics_365_heatmap_50k_rows", |b| {
        b.iter(|| {
            let res = analytics
                .query_365_day_heatmap(Some("engineer_042@gitupload.local"), today)
                .unwrap();
            assert_eq!(res.len(), 365);
        });
    });

    c.bench_function("analytics_leaderboard_100_authors_50k_rows", |b| {
        b.iter(|| {
            let lb = analytics.get_classroom_leaderboard(today).unwrap();
            assert_eq!(lb.len(), 100);
        });
    });
}

criterion_group!(benches, bench_analytics_50k_rows);
criterion_main!(benches);
