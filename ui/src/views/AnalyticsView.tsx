import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  GraphIcon,
  FileDirectoryIcon,
  SyncIcon,
  FlameIcon,
  GitCommitIcon,
  CheckCircleFillIcon,
  ClockIcon,
  AlertFillIcon,
  TrophyIcon,
  PlusIcon,
  TrashIcon,
} from "@primer/octicons-react";
import { ContributionHeatmap } from "../components/ContributionHeatmap";
import { useI18n } from "../i18n/I18nContext";
import {
  formatIpcError,
  type ClassroomAssignment,
  type HeatmapDay,
  type LeaderboardEntry,
  type SubmissionEvaluation,
  type VelocityMetrics,
} from "../types/ipc";

interface AnalyticsViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const AnalyticsView: React.FC<AnalyticsViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [heatmap, setHeatmap] = useState<HeatmapDay[]>([]);
  const [velocity, setVelocity] = useState<VelocityMetrics | null>(null);
  const [assignments, setAssignments] = useState<ClassroomAssignment[]>([]);
  const [evaluations, setEvaluations] = useState<SubmissionEvaluation[]>([]);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [authorFilter, setAuthorFilter] = useState("");

  // Classroom assignment form
  const [sessionNum, setSessionNum] = useState(1);
  const [exerciseNum, setExerciseNum] = useState(1);
  const [assignTitle, setAssignTitle] = useState("Homework Exercise");
  const [deadlineDate, setDeadlineDate] = useState("2026-12-01");

  const [busy, setBusy] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadAnalytics = async () => {
    setErrorMsg(null);
    const author = authorFilter.trim() || null;
    try {
      const [hm, vel, list, evals, lb] = await Promise.all([
        invoke<HeatmapDay[]>("analytics_get_heatmap", { authorEmail: author }),
        invoke<VelocityMetrics>("analytics_get_velocity", {
          authorEmail: author,
        }),
        invoke<ClassroomAssignment[]>("analytics_list_assignments"),
        invoke<SubmissionEvaluation[]>("analytics_evaluate_compliance"),
        invoke<LeaderboardEntry[]>("analytics_get_leaderboard"),
      ]);
      setHeatmap(hm);
      setVelocity(vel);
      setAssignments(list);
      setEvaluations(evals);
      setLeaderboard(lb);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  useEffect(() => {
    loadAnalytics();
  }, []);

  const pickAndScan = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
      await handleScanRepo(selected);
    }
  };

  const handleScanRepo = async (targetPath?: string) => {
    const pathToScan = (targetPath ?? repoPath).trim();
    if (!pathToScan) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      await invoke<number>("analytics_scan_repo", {
        repoPath: pathToScan,
        maxCommits: 10000,
      });
      await loadAnalytics();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleAddAssignment = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!assignTitle.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      await invoke("analytics_upsert_assignment", {
        sessionNum: Number(sessionNum),
        exerciseNum: Number(exerciseNum),
        title: assignTitle.trim(),
        deadlineIso: `${deadlineDate}T23:59:59Z`,
      });
      await loadAnalytics();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleDeleteAssignment = async (id: number) => {
    try {
      await invoke("analytics_delete_assignment", { id });
      await loadAnalytics();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const evalByKey = new Map<string, SubmissionEvaluation>(
    evaluations.map((e) => [`${e.session_num}-${e.exercise_num}`, e])
  );

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <GraphIcon size={20} className="text-accent" />
            {t("analytics.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("analytics.subtitle")}
          </p>
        </div>

        <div className="d-flex flex-items-center gap-2">
          <input
            type="text"
            className="form-control f6"
            placeholder={t("analytics.filterPlaceholder")}
            value={authorFilter}
            onChange={(e) => setAuthorFilter(e.target.value)}
            style={{ width: 190 }}
          />
          <button
            type="button"
            className="btn btn-secondary f6"
            onClick={loadAnalytics}
          >
            {t("common.refresh")}
          </button>
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            onClick={pickAndScan}
          >
            <FileDirectoryIcon size={14} />
            {t("analytics.chooseScan")}
          </button>
          <button
            type="button"
            className="btn btn-primary d-inline-flex flex-items-center gap-1"
            disabled={busy || !repoPath.trim()}
            onClick={() => handleScanRepo()}
          >
            <SyncIcon size={14} />
            {busy ? t("analytics.scanning") : t("analytics.scanActive")}
          </button>
        </div>
      </div>

      {errorMsg && (
        <div
          className="primer-card p-2 text-danger f6"
          style={{ borderColor: "var(--color-danger-fg)" }}
        >
          {errorMsg}
        </div>
      )}

      {/* Velocity KPI Cards */}
      {velocity && (
        <div
          className="d-grid gap-2"
          style={{
            gridTemplateColumns: "repeat(auto-fit, minmax(175px, 1fr))",
          }}
        >
          <div className="primer-card p-3">
            <div className="f6 text-muted d-flex flex-items-center gap-1">
              <GitCommitIcon size={13} />
              30-Day Commits
            </div>
            <div className="f2 text-bold tabular-nums mt-1">
              {velocity.commits_last_30d}
            </div>
            <div className="f6 text-muted tabular-nums">
              {velocity.active_days_30d} active days (
              {velocity.avg_commits_per_active_day.toFixed(1)}/day)
            </div>
          </div>

          <div className="primer-card p-3">
            <div className="f6 text-muted d-flex flex-items-center gap-1">
              <FlameIcon size={13} className="text-attention" />
              Active Streak
            </div>
            <div className="f2 text-bold tabular-nums mt-1 text-attention">
              {velocity.current_streak}d
            </div>
            <div className="f6 text-muted tabular-nums">
              Longest streak: {velocity.longest_streak}d
            </div>
          </div>

          <div className="primer-card p-3">
            <div className="f6 text-muted">Code Additions / Churn</div>
            <div className="f2 text-bold tabular-nums mt-1 text-success">
              +{velocity.total_additions.toLocaleString()}
            </div>
            <div className="f6 text-muted tabular-nums">
              -{velocity.total_deletions.toLocaleString()} ({(velocity.churn_ratio * 100).toFixed(1)}% churn)
            </div>
          </div>

          <div className="primer-card p-3">
            <div className="f6 text-muted d-flex flex-items-center gap-1">
              <CheckCircleFillIcon size={13} className="text-success" />
              Homework On-Time Rate
            </div>
            <div className="f2 text-bold tabular-nums mt-1 text-accent">
              {velocity.on_time_rate.toFixed(0)}%
            </div>
            <div className="f6 text-muted tabular-nums">
              {assignments.length} tracked syllabus tasks
            </div>
          </div>
        </div>
      )}

      {/* 365-Day Heatmap */}
      <ContributionHeatmap days={heatmap} />

      <div className="d-flex gap-3 flex-wrap">
        {/* Classroom Deadline Tracker */}
        <div className="primer-card p-3 flex-1" style={{ minWidth: 320 }}>
          <div className="f5 text-bold d-flex flex-items-center gap-2 mb-2">
            <ClockIcon size={16} className="text-accent" />
            Classroom Syllabus &amp; Deadlines
          </div>

          <form
            onSubmit={handleAddAssignment}
            className="d-flex gap-2 flex-wrap mb-3"
          >
            <input
              type="number"
              min={1}
              className="form-control f6 tabular-nums"
              style={{ width: 64 }}
              value={sessionNum}
              onChange={(e) => setSessionNum(Number(e.target.value))}
              aria-label="Session number"
              title="Session #"
            />
            <input
              type="number"
              min={1}
              className="form-control f6 tabular-nums"
              style={{ width: 64 }}
              value={exerciseNum}
              onChange={(e) => setExerciseNum(Number(e.target.value))}
              aria-label="Exercise number"
              title="Exercise #"
            />
            <input
              type="text"
              className="form-control f6 flex-1"
              value={assignTitle}
              onChange={(e) => setAssignTitle(e.target.value)}
              placeholder="Exercise title..."
            />
            <input
              type="date"
              className="form-control f6"
              value={deadlineDate}
              onChange={(e) => setDeadlineDate(e.target.value)}
            />
            <button
              type="submit"
              className="btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
              disabled={busy}
            >
              <PlusIcon size={13} />
              Add
            </button>
          </form>

          {assignments.length === 0 ? (
            <div className="f6 text-muted">
              No classroom deadlines defined yet. Add an SS/EX syllabus entry above.
            </div>
          ) : (
            <div className="d-flex flex-column gap-1">
              {assignments.map((a) => {
                const ev = evalByKey.get(`${a.session_num}-${a.exercise_num}`);
                const status = ev?.status ?? "missing";
                return (
                  <div
                    key={a.id}
                    className="d-flex flex-justify-between flex-items-center p-2 rounded-2 bg-canvas-inset border border-muted"
                  >
                    <div className="d-flex flex-items-center gap-2 min-width-0">
                      {status === "on_time" ? (
                        <CheckCircleFillIcon size={14} className="text-success" />
                      ) : status === "late" ? (
                        <ClockIcon size={14} className="text-attention" />
                      ) : (
                        <AlertFillIcon size={14} className="text-danger" />
                      )}
                      <span className="text-mono f6 text-bold tabular-nums">
                        SS{String(a.session_num).padStart(2, "0")}-EX
                        {String(a.exercise_num).padStart(2, "0")}
                      </span>
                      <span className="f6 text-truncate">{a.title}</span>
                    </div>
                    <div className="d-flex flex-items-center gap-2 flex-shrink-0">
                      <span className="badge text-capitalize">
                        {status.replace("_", " ")}
                      </span>
                      <span className="f6 text-muted tabular-nums">
                        {a.deadline_utc.slice(0, 10)}
                      </span>
                      <button
                        type="button"
                        className="btn-octicon text-danger"
                        aria-label={`Delete assignment ${a.title}`}
                        onClick={() => handleDeleteAssignment(a.id)}
                      >
                        <TrashIcon size={13} />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Team / Classroom Leaderboard */}
        <div className="primer-card p-3 flex-1" style={{ minWidth: 300 }}>
          <div className="f5 text-bold d-flex flex-items-center gap-2 mb-2">
            <TrophyIcon size={16} className="text-attention" />
            Contributor Productivity Leaderboard
          </div>

          {leaderboard.length === 0 ? (
            <div className="empty-state-card">
              <TrophyIcon size={24} className="text-muted mb-2" />
              <div className="f6 text-muted">
                Scan a local repository to populate contributor velocity scores.
              </div>
            </div>
          ) : (
            <div className="d-flex flex-column gap-1">
              {leaderboard.map((row) => (
                <div
                  key={row.author_email}
                  className="d-flex flex-justify-between flex-items-center p-2 rounded-2 bg-canvas-inset border border-muted interactive-row"
                >
                  <div className="d-flex flex-items-center gap-2 min-width-0">
                    <span className="badge text-bold tabular-nums">
                      #{row.rank}
                    </span>
                    <span className="f6 text-truncate text-mono">
                      {row.author_email}
                    </span>
                  </div>
                  <div className="d-flex flex-items-center gap-3 f6 tabular-nums flex-shrink-0">
                    <span className="text-muted">
                      {row.total_commits} commits
                    </span>
                    <span className="text-attention">
                      {row.current_streak}d streak
                    </span>
                    <strong className="text-success">{row.score} pts</strong>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default AnalyticsView;
