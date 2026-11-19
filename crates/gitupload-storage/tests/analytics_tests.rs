use chrono::{Duration, NaiveDate, TimeZone, Utc};
use gitupload_core::models::{DailyCommitStat, PushHistoryRecord, SubmissionStatus};
use gitupload_storage::{AnalyticsRepository, HistoryRepository, StoragePool};

#[test]
fn test_heatmap_velocity_streaks_and_classroom_compliance() {
    let pool = StoragePool::in_memory().unwrap();
    let analytics = AnalyticsRepository::new(&pool);
    let history = HistoryRepository::new(&pool);

    let today = NaiveDate::from_ymd_opt(2026, 11, 19).unwrap();

    // Seed 5 consecutive days ending today + peak activity
    let mut stats = Vec::new();
    for offset in 0..5 {
        let day = (today - Duration::days(offset))
            .format("%Y-%m-%d")
            .to_string();
        let commits = if offset == 0 { 10 } else { (5 - offset) as u32 };
        stats.push(DailyCommitStat {
            date: day,
            repo_name: "session01-ex01-homework".to_string(),
            commit_count: commits,
            lines_added: 120,
            lines_deleted: 30,
            author_email: "student@gitupload.local".to_string(),
        });
    }
    analytics.upsert_daily_stats_batch(&stats).unwrap();

    let heatmap = analytics
        .query_365_day_heatmap(Some("student@gitupload.local"), today)
        .unwrap();
    assert_eq!(heatmap.len(), 365);
    assert_eq!(heatmap[364].count, 10);
    assert_eq!(heatmap[364].tier, 4);
    assert_eq!(heatmap[0].count, 0);
    assert_eq!(heatmap[0].tier, 0);

    let vel = analytics
        .calculate_velocity_and_streaks(Some("student@gitupload.local"), today)
        .unwrap();
    assert_eq!(vel.current_streak, 5);
    assert_eq!(vel.longest_streak, 5);
    assert_eq!(vel.active_days_30d, 5);
    assert_eq!(vel.commits_last_30d, 10 + 4 + 3 + 2 + 1);
    assert!((vel.churn_ratio - 0.25).abs() < 0.001);

    // Classroom deadline evaluation
    let deadline_early = Utc.with_ymd_and_hms(2026, 11, 18, 12, 0, 0).unwrap();
    let deadline_future = Utc.with_ymd_and_hms(2026, 11, 25, 23, 59, 59).unwrap();
    analytics
        .upsert_assignment(1, 1, "Intro Repo", deadline_future)
        .unwrap();
    analytics
        .upsert_assignment(1, 2, "Missed Deadline Repo", deadline_early)
        .unwrap();
    analytics
        .upsert_assignment(2, 1, "Unpushed Exercise", deadline_future)
        .unwrap();

    history
        .save(&PushHistoryRecord {
            id: "push-1".into(),
            account_id: "default".into(),
            repo_name: "session01-ex01-homework".into(),
            repo_url: "https://github.com/u/session01-ex01-homework".into(),
            local_path: "C:\\work\\ss01".into(),
            commit_sha: Some("aaa".into()),
            commit_msg: "feat(ss01-ex01): submit on time".into(),
            files_count: 3,
            status: "SUCCESS".into(),
            error_message: None,
            is_force_with_lease: false,
            duration_ms: 100,
            created_at: Utc.with_ymd_and_hms(2026, 11, 19, 8, 0, 0).unwrap(),
        })
        .unwrap();

    history
        .save(&PushHistoryRecord {
            id: "push-2".into(),
            account_id: "default".into(),
            repo_name: "session01-ex02-homework".into(),
            repo_url: "https://github.com/u/session01-ex02-homework".into(),
            local_path: "C:\\work\\ss01ex02".into(),
            commit_sha: Some("bbb".into()),
            commit_msg: "feat(ss01-ex02): late submit".into(),
            files_count: 2,
            status: "SUCCESS".into(),
            error_message: None,
            is_force_with_lease: false,
            duration_ms: 100,
            created_at: Utc.with_ymd_and_hms(2026, 11, 19, 9, 0, 0).unwrap(),
        })
        .unwrap();

    let evals = analytics.evaluate_classroom_compliance().unwrap();
    assert_eq!(evals.len(), 3);
    assert_eq!(evals[0].status, SubmissionStatus::OnTime);
    assert_eq!(evals[1].status, SubmissionStatus::Late);
    assert_eq!(evals[2].status, SubmissionStatus::Missing);

    let lb = analytics.get_classroom_leaderboard(today).unwrap();
    assert_eq!(lb.len(), 1);
    assert_eq!(lb[0].rank, 1);
    assert_eq!(lb[0].total_commits, 20);
}
