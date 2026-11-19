//! Team analytics, 365-day contribution heatmap, commit velocity, and classroom leaderboard DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Aggregated commit telemetry record grouped by `(date, repo_name, author_email)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyCommitStat {
    /// Calendar date (`YYYY-MM-DD`, UTC).
    pub date: String,
    /// Repository name or identifier.
    pub repo_name: String,
    /// Number of commits recorded on this day.
    pub commit_count: u32,
    /// Total inserted lines.
    pub lines_added: u64,
    /// Total deleted lines.
    pub lines_deleted: u64,
    /// Contributor email address.
    pub author_email: String,
}

/// Single day cell in the 365-day GitHub-fidelity contribution heatmap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeatmapDay {
    /// Calendar date (`YYYY-MM-DD`).
    pub date: String,
    /// Total commit count on that day.
    pub count: u32,
    /// GitHub 5-tier intensity level (`0` empty .. `4` peak).
    pub tier: u8,
}

/// Computed velocity, code churn, streak, and assignment compliance metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VelocityMetrics {
    /// Total commits in the last 30 days.
    pub commits_last_30d: u32,
    /// Number of distinct active coding days in the last 30 days.
    pub active_days_30d: u32,
    /// Average commits per active day.
    pub avg_commits_per_active_day: f64,
    /// Cumulative lines added.
    pub total_additions: u64,
    /// Cumulative lines deleted.
    pub total_deletions: u64,
    /// Code churn ratio (`deletions / max(1, additions)`).
    pub churn_ratio: f64,
    /// Current consecutive active day streak ending today or yesterday.
    pub current_streak: u32,
    /// Longest historical consecutive day streak.
    pub longest_streak: u32,
    /// Percentage (`0.0..=100.0`) of classroom exercises submitted before deadline.
    pub on_time_rate: f64,
}

/// Classroom homework assignment syllabus definition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassroomAssignment {
    /// Database row ID.
    pub id: i64,
    /// Course session number (`SS01`).
    pub session_num: u32,
    /// Exercise number (`EX02`).
    pub exercise_num: u32,
    /// Descriptive title.
    pub title: String,
    /// Submission deadline in UTC.
    pub deadline_utc: DateTime<Utc>,
}

/// Deadline timeliness classification for a classroom assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionStatus {
    /// Pushed at or before `deadline_utc`.
    OnTime,
    /// Pushed after `deadline_utc`.
    Late,
    /// No matching submission push recorded yet.
    Missing,
}

/// Evaluation summary for one classroom assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmissionEvaluation {
    /// Course session number.
    pub session_num: u32,
    /// Exercise number.
    pub exercise_num: u32,
    /// Assignment title.
    pub title: String,
    /// Timeliness classification.
    pub status: SubmissionStatus,
    /// Earliest matching push timestamp if submitted.
    pub submitted_at: Option<DateTime<Utc>>,
}

/// Ranked entry on the classroom / engineering team leaderboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaderboardEntry {
    /// 1-based rank position.
    pub rank: usize,
    /// Contributor email or handle.
    pub author_email: String,
    /// Total commits recorded.
    pub total_commits: u32,
    /// Current consecutive coding streak (days).
    pub current_streak: u32,
    /// Ratio (`0.0..=1.0`) of clean Conventional Commits pushes.
    pub conventional_ratio: f64,
    /// Count of on-time assignment pushes.
    pub on_time_count: u32,
    /// Composite productivity score.
    pub score: u64,
}
