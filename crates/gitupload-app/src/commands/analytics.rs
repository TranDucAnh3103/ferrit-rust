//! Team analytics, 365-day heatmap, velocity/churn KPIs, classroom compliance & leaderboard IPC commands (`analytics_*`).

use crate::commands::validate_safe_path;
use crate::state::AppState;
use chrono::{DateTime, Utc};
use gitupload_core::errors::{GitUploadError, IpcErrorPayload};
use gitupload_core::models::{
    ClassroomAssignment, HeatmapDay, LeaderboardEntry, SubmissionEvaluation, VelocityMetrics,
};
use gitupload_git::CommitAggregator;
use gitupload_storage::AnalyticsRepository;
use tauri::State;

/// Scans local Git commit history via `libgit2` revwalk and persists daily commit telemetry into SQLite.
#[tauri::command]
pub async fn analytics_scan_repo(
    state: State<'_, AppState>,
    repo_path: String,
    max_commits: Option<usize>,
) -> Result<usize, IpcErrorPayload> {
    let safe_path = validate_safe_path(&repo_path)?;
    let limit = max_commits.unwrap_or(10_000);
    let stats =
        tokio::task::spawn_blocking(move || CommitAggregator::scan_repository(&safe_path, limit))
            .await
            .map_err(|e| GitUploadError::Internal(e.to_string()))?
            .map_err(IpcErrorPayload::from)?;

    AnalyticsRepository::new(&state.storage)
        .upsert_daily_stats_batch(&stats)
        .map_err(IpcErrorPayload::from)
}

/// Returns the 365-day continuous GitHub-fidelity contribution heatmap ending today UTC.
#[tauri::command]
pub async fn analytics_get_heatmap(
    state: State<'_, AppState>,
    author_email: Option<String>,
) -> Result<Vec<HeatmapDay>, IpcErrorPayload> {
    let today = Utc::now().date_naive();
    AnalyticsRepository::new(&state.storage)
        .query_365_day_heatmap(author_email.as_deref(), today)
        .map_err(IpcErrorPayload::from)
}

/// Returns rolling 30-day commit velocity, code churn, active streaks, and classroom compliance.
#[tauri::command]
pub async fn analytics_get_velocity(
    state: State<'_, AppState>,
    author_email: Option<String>,
) -> Result<VelocityMetrics, IpcErrorPayload> {
    let today = Utc::now().date_naive();
    AnalyticsRepository::new(&state.storage)
        .calculate_velocity_and_streaks(author_email.as_deref(), today)
        .map_err(IpcErrorPayload::from)
}

/// Lists all classroom homework syllabus assignments.
#[tauri::command]
pub async fn analytics_list_assignments(
    state: State<'_, AppState>,
) -> Result<Vec<ClassroomAssignment>, IpcErrorPayload> {
    AnalyticsRepository::new(&state.storage)
        .list_assignments()
        .map_err(IpcErrorPayload::from)
}

/// Creates or updates a classroom homework deadline entry (`SSxx / EXyy`).
#[tauri::command]
pub async fn analytics_upsert_assignment(
    state: State<'_, AppState>,
    session_num: u32,
    exercise_num: u32,
    title: String,
    deadline_iso: String,
) -> Result<ClassroomAssignment, IpcErrorPayload> {
    let parsed_dt = DateTime::parse_from_rfc3339(deadline_iso.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            IpcErrorPayload::from(GitUploadError::Config(format!(
                "Invalid ISO-8601 deadline timestamp '{deadline_iso}': {e}"
            )))
        })?;

    AnalyticsRepository::new(&state.storage)
        .upsert_assignment(session_num, exercise_num, &title, parsed_dt)
        .map_err(IpcErrorPayload::from)
}

/// Deletes a classroom homework assignment by ID.
#[tauri::command]
pub async fn analytics_delete_assignment(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), IpcErrorPayload> {
    AnalyticsRepository::new(&state.storage)
        .delete_assignment(id)
        .map_err(IpcErrorPayload::from)
}

/// Evaluates on-time vs late vs missing classroom submission compliance against `push_history`.
#[tauri::command]
pub async fn analytics_evaluate_compliance(
    state: State<'_, AppState>,
) -> Result<Vec<SubmissionEvaluation>, IpcErrorPayload> {
    AnalyticsRepository::new(&state.storage)
        .evaluate_classroom_compliance()
        .map_err(IpcErrorPayload::from)
}

/// Returns the ranked contributor & classroom productivity leaderboard.
#[tauri::command]
pub async fn analytics_get_leaderboard(
    state: State<'_, AppState>,
) -> Result<Vec<LeaderboardEntry>, IpcErrorPayload> {
    let today = Utc::now().date_naive();
    AnalyticsRepository::new(&state.storage)
        .get_classroom_leaderboard(today)
        .map_err(IpcErrorPayload::from)
}
