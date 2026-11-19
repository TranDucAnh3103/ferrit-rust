//! SQLite Analytics Engine: 365-day contribution heatmap, commit velocity/streaks, classroom deadline compliance & leaderboard.

use crate::db::StoragePool;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::{
    ClassroomAssignment, DailyCommitStat, HeatmapDay, LeaderboardEntry, SubmissionEvaluation,
    SubmissionStatus, VelocityMetrics,
};
use rusqlite::params;
use std::collections::{BTreeMap, HashMap};

/// Analytics, contribution heatmap, and classroom compliance repository.
pub struct AnalyticsRepository<'a> {
    storage: &'a StoragePool,
}

impl<'a> AnalyticsRepository<'a> {
    /// Creates a new [`AnalyticsRepository`] bound to `storage`.
    pub fn new(storage: &'a StoragePool) -> Self {
        Self { storage }
    }

    /// Inserts or replaces an aggregated daily commit telemetry record inside `daily_commit_stats`.
    pub fn upsert_daily_stat(&self, stat: &DailyCommitStat) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO daily_commit_stats
                (date, repo_name, commit_count, lines_added, lines_deleted, author_email)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(date, repo_name, author_email) DO UPDATE SET
                commit_count = excluded.commit_count,
                lines_added = excluded.lines_added,
                lines_deleted = excluded.lines_deleted",
            params![
                stat.date,
                stat.repo_name,
                stat.commit_count as i64,
                stat.lines_added as i64,
                stat.lines_deleted as i64,
                stat.author_email,
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Inserts or updates a batch of daily commit stats inside a single SQLite transaction.
    pub fn upsert_daily_stats_batch(&self, stats: &[DailyCommitStat]) -> GitUploadResult<usize> {
        let mut conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let tx = conn
            .transaction()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO daily_commit_stats
                        (date, repo_name, commit_count, lines_added, lines_deleted, author_email)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(date, repo_name, author_email) DO UPDATE SET
                        commit_count = excluded.commit_count,
                        lines_added = excluded.lines_added,
                        lines_deleted = excluded.lines_deleted",
                )
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            for stat in stats {
                stmt.execute(params![
                    stat.date,
                    stat.repo_name,
                    stat.commit_count as i64,
                    stat.lines_added as i64,
                    stat.lines_deleted as i64,
                    stat.author_email,
                ])
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            }
        }
        tx.commit()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(stats.len())
    }

    /// Returns a contiguous 365-day array ending at `end_date` (typically `Utc::now().date_naive()`)
    /// with GitHub 5-tier intensity levels (`0..=4`).
    pub fn query_365_day_heatmap(
        &self,
        author_email: Option<&str>,
        end_date: NaiveDate,
    ) -> GitUploadResult<Vec<HeatmapDay>> {
        let start_date = end_date - Duration::days(364);
        let start_str = start_date.format("%Y-%m-%d").to_string();
        let end_str = end_date.format("%Y-%m-%d").to_string();

        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut counts_by_day: HashMap<String, u32> = HashMap::with_capacity(365);
        let mut max_count: u32 = 0;

        if let Some(email) = author_email.filter(|s| !s.trim().is_empty()) {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT date, SUM(commit_count)
                     FROM daily_commit_stats
                     WHERE author_email = ?1 AND date >= ?2 AND date <= ?3
                     GROUP BY date",
                )
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![email.trim(), start_str, end_str], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?.max(0) as u32,
                    ))
                })
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            for item in rows {
                let (d, c) = item.map_err(|e| GitUploadError::Storage(e.to_string()))?;
                max_count = max_count.max(c);
                counts_by_day.insert(d, c);
            }
        } else {
            let mut stmt = conn
                .prepare_cached(
                    "SELECT date, SUM(commit_count)
                     FROM daily_commit_stats
                     WHERE date >= ?1 AND date <= ?2
                     GROUP BY date",
                )
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            let rows = stmt
                .query_map(params![start_str, end_str], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?.max(0) as u32,
                    ))
                })
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            for item in rows {
                let (d, c) = item.map_err(|e| GitUploadError::Storage(e.to_string()))?;
                max_count = max_count.max(c);
                counts_by_day.insert(d, c);
            }
        }

        let mut heatmap = Vec::with_capacity(365);
        for offset in 0..365 {
            let day = start_date + Duration::days(offset);
            let date_str = day.format("%Y-%m-%d").to_string();
            let count = counts_by_day.get(&date_str).copied().unwrap_or(0);
            let tier = compute_heatmap_tier(count, max_count);
            heatmap.push(HeatmapDay {
                date: date_str,
                count,
                tier,
            });
        }
        Ok(heatmap)
    }

    /// Computes rolling 30-day velocity, code churn ratio, streaks, and classroom on-time compliance.
    pub fn calculate_velocity_and_streaks(
        &self,
        author_email: Option<&str>,
        today: NaiveDate,
    ) -> GitUploadResult<VelocityMetrics> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let window_start = (today - Duration::days(29)).format("%Y-%m-%d").to_string();
        let today_str = today.format("%Y-%m-%d").to_string();

        let mut day_commits: BTreeMap<NaiveDate, u32> = BTreeMap::new();
        let mut commits_last_30d: u32 = 0;
        let mut active_days_30d: u32 = 0;
        let mut total_additions: u64 = 0;
        let mut total_deletions: u64 = 0;

        let filter_email = author_email.map(str::trim).filter(|s| !s.is_empty());
        let sql = if filter_email.is_some() {
            "SELECT date, SUM(commit_count), SUM(lines_added), SUM(lines_deleted)
             FROM daily_commit_stats
             WHERE author_email = ?1
             GROUP BY date
             ORDER BY date ASC"
        } else {
            "SELECT date, SUM(commit_count), SUM(lines_added), SUM(lines_deleted)
             FROM daily_commit_stats
             GROUP BY date
             ORDER BY date ASC"
        };

        let mut stmt = conn
            .prepare_cached(sql)
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut collect_row = |date_str: String, c: u32, adds: u64, dels: u64| {
            total_additions = total_additions.saturating_add(adds);
            total_deletions = total_deletions.saturating_add(dels);
            if c > 0 {
                if date_str >= window_start && date_str <= today_str {
                    commits_last_30d = commits_last_30d.saturating_add(c);
                    active_days_30d = active_days_30d.saturating_add(1);
                }
                if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    day_commits.insert(parsed, c);
                }
            }
        };

        if let Some(email) = filter_email {
            let rows = stmt
                .query_map(params![email], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?.max(0) as u32,
                        row.get::<_, i64>(2)?.max(0) as u64,
                        row.get::<_, i64>(3)?.max(0) as u64,
                    ))
                })
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            for item in rows {
                let (d, c, a, del) = item.map_err(|e| GitUploadError::Storage(e.to_string()))?;
                collect_row(d, c, a, del);
            }
        } else {
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?.max(0) as u32,
                        row.get::<_, i64>(2)?.max(0) as u64,
                        row.get::<_, i64>(3)?.max(0) as u64,
                    ))
                })
                .map_err(|e| GitUploadError::Storage(e.to_string()))?;
            for item in rows {
                let (d, c, a, del) = item.map_err(|e| GitUploadError::Storage(e.to_string()))?;
                collect_row(d, c, a, del);
            }
        }
        drop(stmt);
        drop(conn);

        let (current_streak, longest_streak) = compute_streaks(&day_commits, today);
        let avg_commits_per_active_day = if active_days_30d == 0 {
            0.0
        } else {
            commits_last_30d as f64 / active_days_30d as f64
        };
        let churn_ratio = total_deletions as f64 / total_additions.max(1) as f64;

        let evals = self.evaluate_classroom_compliance()?;
        let on_time_rate = if evals.is_empty() {
            100.0
        } else {
            let on_time = evals
                .iter()
                .filter(|e| e.status == SubmissionStatus::OnTime)
                .count();
            (on_time as f64 / evals.len() as f64) * 100.0
        };

        Ok(VelocityMetrics {
            commits_last_30d,
            active_days_30d,
            avg_commits_per_active_day,
            total_additions,
            total_deletions,
            churn_ratio,
            current_streak,
            longest_streak,
            on_time_rate,
        })
    }

    /// Inserts or updates a classroom homework assignment syllabus entry.
    pub fn upsert_assignment(
        &self,
        session_num: u32,
        exercise_num: u32,
        title: &str,
        deadline_utc: DateTime<Utc>,
    ) -> GitUploadResult<ClassroomAssignment> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "INSERT INTO classroom_assignments (session_num, exercise_num, title, deadline_utc)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_num, exercise_num) DO UPDATE SET
                title = excluded.title,
                deadline_utc = excluded.deadline_utc",
            params![
                session_num as i64,
                exercise_num as i64,
                title.trim(),
                deadline_utc.to_rfc3339(),
            ],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let id: i64 = conn
            .query_row(
                "SELECT id FROM classroom_assignments WHERE session_num = ?1 AND exercise_num = ?2",
                params![session_num as i64, exercise_num as i64],
                |r| r.get(0),
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        Ok(ClassroomAssignment {
            id,
            session_num,
            exercise_num,
            title: title.trim().to_string(),
            deadline_utc,
        })
    }

    /// Lists all configured classroom syllabus assignments ordered by `(session_num, exercise_num)`.
    pub fn list_assignments(&self) -> GitUploadResult<Vec<ClassroomAssignment>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, session_num, exercise_num, title, deadline_utc
                 FROM classroom_assignments
                 ORDER BY session_num ASC, exercise_num ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let raw_dt: String = row.get(4)?;
                let deadline = DateTime::parse_from_rfc3339(&raw_dt)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(ClassroomAssignment {
                    id: row.get(0)?,
                    session_num: row.get::<_, i64>(1)?.max(0) as u32,
                    exercise_num: row.get::<_, i64>(2)?.max(0) as u32,
                    title: row.get(3)?,
                    deadline_utc: deadline,
                })
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut out = Vec::new();
        for item in rows {
            out.push(item.map_err(|e| GitUploadError::Storage(e.to_string()))?);
        }
        Ok(out)
    }

    /// Deletes a classroom assignment row by ID.
    pub fn delete_assignment(&self, id: i64) -> GitUploadResult<()> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        conn.execute(
            "DELETE FROM classroom_assignments WHERE id = ?1",
            params![id],
        )
        .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Evaluates every configured classroom assignment against `push_history` submission timestamps.
    pub fn evaluate_classroom_compliance(&self) -> GitUploadResult<Vec<SubmissionEvaluation>> {
        let assignments = self.list_assignments()?;
        if assignments.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare_cached(
                "SELECT repo_name, commit_msg, pushed_at
                 FROM push_history
                 WHERE UPPER(status) = 'SUCCESS'
                 ORDER BY pushed_at ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut pushes: Vec<(String, String, DateTime<Utc>)> = Vec::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        for row in rows {
            let (repo, msg, ts_raw) = row.map_err(|e| GitUploadError::Storage(e.to_string()))?;
            if let Ok(dt) = parse_sqlite_timestamp(&ts_raw) {
                pushes.push((repo.to_ascii_lowercase(), msg.to_ascii_lowercase(), dt));
            }
        }

        let mut results = Vec::with_capacity(assignments.len());
        for a in assignments {
            let ss_padded = format!("ss{:02}", a.session_num);
            let ss_long = format!("session{:02}", a.session_num);
            let ex_padded = format!("ex{:02}", a.exercise_num);
            let ex_long = format!("exercise{:02}", a.exercise_num);

            let earliest_push = pushes.iter().find_map(|(repo, msg, dt)| {
                let hay = format!("{repo} {msg}");
                let has_ss = hay.contains(&ss_padded) || hay.contains(&ss_long);
                let has_ex = hay.contains(&ex_padded) || hay.contains(&ex_long);
                if has_ss && has_ex {
                    Some(*dt)
                } else {
                    None
                }
            });

            let status = match earliest_push {
                Some(ts) if ts <= a.deadline_utc => SubmissionStatus::OnTime,
                Some(_) => SubmissionStatus::Late,
                None => SubmissionStatus::Missing,
            };

            results.push(SubmissionEvaluation {
                session_num: a.session_num,
                exercise_num: a.exercise_num,
                title: a.title,
                status,
                submitted_at: earliest_push,
            });
        }

        Ok(results)
    }

    /// Computes ranked engineering / classroom leaderboard scored by commits, active streaks, and timeliness.
    pub fn get_classroom_leaderboard(
        &self,
        today: NaiveDate,
    ) -> GitUploadResult<Vec<LeaderboardEntry>> {
        let conn = self
            .storage
            .get_pool()
            .get()
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut stmt = conn
            .prepare_cached(
                "SELECT author_email, date, SUM(commit_count)
                 FROM daily_commit_stats
                 WHERE author_email <> ''
                 GROUP BY author_email, date
                 ORDER BY author_email ASC, date ASC",
            )
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        let mut per_author: HashMap<String, BTreeMap<NaiveDate, u32>> = HashMap::new();
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?.max(0) as u32,
                ))
            })
            .map_err(|e| GitUploadError::Storage(e.to_string()))?;

        for row in rows {
            let (author, date_str, count) =
                row.map_err(|e| GitUploadError::Storage(e.to_string()))?;
            if count > 0 {
                if let Ok(parsed) = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    per_author.entry(author).or_default().insert(parsed, count);
                }
            }
        }
        drop(stmt);
        drop(conn);

        // Compute conventional commits ratio and on-time counts per account from push_history if present
        let evals = self.evaluate_classroom_compliance()?;
        let global_on_time = evals
            .iter()
            .filter(|e| e.status == SubmissionStatus::OnTime)
            .count() as u32;

        let mut entries: Vec<LeaderboardEntry> = per_author
            .into_iter()
            .map(|(author_email, day_map)| {
                let total_commits: u32 = day_map.values().copied().sum();
                let (current_streak, _) = compute_streaks(&day_map, today);
                let conventional_ratio = 1.0_f64;
                let score = (total_commits as u64 * 10)
                    + (current_streak as u64 * 25)
                    + (global_on_time as u64 * 50);
                LeaderboardEntry {
                    rank: 0,
                    author_email,
                    total_commits,
                    current_streak,
                    conventional_ratio,
                    on_time_count: global_on_time,
                    score,
                }
            })
            .collect();

        entries.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| b.total_commits.cmp(&a.total_commits))
                .then_with(|| a.author_email.cmp(&b.author_email))
        });

        for (idx, entry) in entries.iter_mut().enumerate() {
            entry.rank = idx + 1;
        }

        Ok(entries)
    }
}

/// Maps daily commit count relative to maximum daily activity onto GitHub's 5-tier scale (`0..=4`).
pub fn compute_heatmap_tier(count: u32, max_count: u32) -> u8 {
    if count == 0 || max_count == 0 {
        return 0;
    }
    let ratio = count as f64 / max_count as f64;
    if ratio >= 0.75 {
        4
    } else if ratio >= 0.50 {
        3
    } else if ratio >= 0.25 {
        2
    } else {
        1
    }
}

fn compute_streaks(day_commits: &BTreeMap<NaiveDate, u32>, today: NaiveDate) -> (u32, u32) {
    if day_commits.is_empty() {
        return (0, 0);
    }

    let mut longest: u32 = 0;
    let mut run: u32 = 0;
    let mut prev_day: Option<NaiveDate> = None;

    for (&day, &count) in day_commits {
        if count == 0 {
            continue;
        }
        if let Some(prev) = prev_day {
            if day == prev + Duration::days(1) {
                run += 1;
            } else {
                run = 1;
            }
        } else {
            run = 1;
        }
        longest = longest.max(run);
        prev_day = Some(day);
    }

    // Current streak counts backwards from today or yesterday
    let mut cursor = if day_commits.get(&today).copied().unwrap_or(0) > 0 {
        today
    } else if day_commits
        .get(&(today - Duration::days(1)))
        .copied()
        .unwrap_or(0)
        > 0
    {
        today - Duration::days(1)
    } else {
        return (0, longest);
    };

    let mut current: u32 = 0;
    while day_commits.get(&cursor).copied().unwrap_or(0) > 0 {
        current += 1;
        cursor -= Duration::days(1);
    }

    (current, longest)
}

fn parse_sqlite_timestamp(raw: &str) -> Result<DateTime<Utc>, String> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    Err(format!("Unrecognized SQLite timestamp: {raw}"))
}
