# Sprint 2 — Phase 4: Team Analytics, Contribution Heatmap & Classroom Metrics Specification
**Document Identifier:** SPRINT2-PHASE4-TEAM-ANALYTICS-HEATMAP  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** Relational SQLite Analytics Schema, 365-Day Contribution Heatmap Data Service, Commit Velocity & Code Churn Calculations, Classroom Attendance/Submission Tracking, and Team Leaderboard  
**Target Crates:** `gitupload-core`, `gitupload-storage`, `gitupload-git`, `gitupload-app`, `ui/`  
**Timeline:** Days 16–20 (Sprint 2 Team Productivity & Academic Insights)  

---

## 1. Phase Overview & Objectives

In academic and software engineering team environments, tracking individual student progress, commit consistency, and assignment completion velocity is vital for instructors and team leads. The legacy Go tool only kept a single JSON entry of the last pushed repository, erasing all historical telemetry.

Phase 4 introduces a **Comprehensive Team Analytics & Heatmap Subsystem**. It aggregates Git commit logs, homework submissions, and pull requests into high-performance relational tables in SQLite. The frontend renders an authentic **365-day GitHub contribution heatmap**, commit velocity graphs, and classroom submission leaderboards with zero external tracking servers or cloud dependencies.

### Core Objectives:
1. **Relational Analytics Schema:** Index historical commits, file changes, and student homework records in SQLite for instant query execution (< 5ms).
2. **GitHub-Fidelity 365-Day Contribution Heatmap:** Aggregate daily commit frequencies into 5 color intensity tiers (Empty `#161b22`, Level 1 `#0e4429`, Level 2 `#006d32`, Level 3 `#26a641`, Level 4 `#39d353`) matching GitHub's web design.
3. **Commit Velocity & Code Churn Metrics:** Calculate additions, deletions, net line changes, and time-of-day commit distributions to evaluate team coding pace.
4. **Classroom Homework Submission Tracker:** Match session and exercise metadata against a classroom syllabus roster to compute on-time submission percentages, missing exercises, and late deliveries.
5. **Team Leaderboard & Achievements:** Calculate gamified productivity scores (streak days, quality commits passing conventional hooks, zero conflict merges) for classroom engagement.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 4 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |        TASK-S2P4-01         |  |         TASK-S2P4-02          |  |       TASK-S2P4-03      |  |
|  | Relational Analytics Schema |  | In-Process Git Log Aggregator |  | 365-Day Heatmap Service |  |
|  | (Daily Commits, Rosters)    |  | (Lines +/- & Commit Times)    |  | (5-Tier GitHub Colors)  |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |   Team Analytics Subsystem   |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |        TASK-S2P4-04         |   |         TASK-S2P4-05          |   |       TASK-S2P4-06     |  |
|  | Velocity & Churn Calculator |   | Classroom Leaderboard Engine  |   | Primer UI Analytics     |  |
|  | (On-Time / Late Deliveries) |   | (Streak Days & Scoring)       |   | Dashboard & Heatmap     |  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-S2P4-01: Relational Analytics Schema DDL & Aggregation Indices

- **Task Identifier:** `TASK-S2P4-01`
- **Component:** `crates/gitupload-storage/src/schema/analytics.rs`
- **SRS Traceability:** `FR-SET-001`, `NFR-PERF-003`

#### Implementation Specification
Execute relational migration defining analytical tables with optimized composite indices:

```sql
-- crates/gitupload-storage/src/schema/analytics.rs (Migration v3)
CREATE TABLE IF NOT EXISTS daily_commit_stats (
    date TEXT NOT NULL, -- YYYY-MM-DD
    repo_name TEXT NOT NULL,
    commit_count INTEGER NOT NULL DEFAULT 0,
    lines_added INTEGER NOT NULL DEFAULT 0,
    lines_deleted INTEGER NOT NULL DEFAULT 0,
    author_email TEXT NOT NULL,
    PRIMARY KEY(date, repo_name, author_email)
);

CREATE TABLE IF NOT EXISTS classroom_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_num INTEGER NOT NULL,
    exercise_num INTEGER NOT NULL,
    title TEXT NOT NULL,
    deadline_utc DATETIME NOT NULL,
    UNIQUE(session_num, exercise_num)
);

CREATE INDEX IF NOT EXISTS idx_commit_stats_date ON daily_commit_stats(date);
CREATE INDEX IF NOT EXISTS idx_commit_stats_author ON daily_commit_stats(author_email);
```

---

### Task TASK-S2P4-02: In-Process Git Log Analytics Aggregator

- **Task Identifier:** `TASK-S2P4-02`
- **Component:** `crates/gitupload-git/src/analytics/aggregator.rs`
- **SRS Traceability:** `FR-GIT-006`, `FR-GRF-001`

#### Implementation Specification
Scan local Git commit history via `git2::Repository`:
- Extract diff stats (`stats.insertions()`, `stats.deletions()`) for each commit.
- Extract commit timestamp and author identity.
- Batch upsert into `daily_commit_stats` in a single SQLite transaction.

```rust
// crates/gitupload-git/src/analytics/aggregator.rs
use git2::{DiffOptions, Repository};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use std::path::Path;

pub struct CommitAggregator;

#[derive(Debug, Clone)]
pub struct CommitRecord {
    pub date: String,
    pub repo_name: String,
    pub author_email: String,
    pub lines_added: usize,
    pub lines_deleted: usize,
}

impl CommitAggregator {
    pub fn scan_repository(repo_path: &Path, repo_name: &str) -> GitUploadResult<Vec<CommitRecord>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repo: {}", e)))?;

        let mut revwalk = repo.revwalk().map_err(|e| GitUploadError::Git(e.to_string()))?;
        revwalk.push_head().ok();

        let mut records = Vec::new();

        for oid in revwalk.filter_map(|r| r.ok()) {
            if let Ok(commit) = repo.find_commit(oid) {
                let date = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "1970-01-01".into());

                let author_email = commit.author().email().unwrap_or("unknown").to_string();

                // Compute diff stats against first parent (if any)
                let (added, deleted) = if let Ok(parent) = commit.parent(0) {
                    if let (Ok(p_tree), Ok(c_tree)) = (parent.tree(), commit.tree()) {
                        let mut opts = DiffOptions::new();
                        if let Ok(diff) = repo.diff_tree_to_tree(Some(&p_tree), Some(&c_tree), Some(&mut opts)) {
                            diff.stats().map(|s| (s.insertions(), s.deletions())).unwrap_or((0, 0))
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                };

                records.push(CommitRecord {
                    date,
                    repo_name: repo_name.to_string(),
                    author_email,
                    lines_added: added,
                    lines_deleted: deleted,
                });
            }
        }

        Ok(records)
    }
}
```

---

### Task TASK-S2P4-03: 365-Day Contribution Heatmap Data Service

- **Task Identifier:** `TASK-S2P4-03`
- **Component:** `crates/gitupload-storage/src/repositories/analytics.rs`
- **SRS Traceability:** `FR-SET-001`, `NFR-PERF-003`

#### Implementation Specification
Compute continuous 365-day calendar grid:
- Query `SELECT date, SUM(commit_count) FROM daily_commit_stats WHERE date >= ? GROUP BY date`.
- Fill missing days with count 0.
- Map count to level: 0 = Tier 0, 1–3 = Tier 1, 4–6 = Tier 2, 7–9 = Tier 3, 10+ = Tier 4.

---

### Task TASK-S2P4-04: Commit Velocity, Code Churn & Deadline Tracking

- **Task Identifier:** `TASK-S2P4-04`
- **Component:** `crates/gitupload-storage/src/repositories/velocity.rs`
- **SRS Traceability:** `FR-GIT-005`, `FR-SET-001`

#### Implementation Specification
Compute performance indicators:
- **Velocity:** Average commits per active coding day over 30 days.
- **Code Churn:** Ratio of deleted lines to added lines.
- **On-Time Rate:** Percentage of homework pushed before `classroom_assignments.deadline_utc`.

---

### Task TASK-S2P4-05: Team Leaderboard & Achievement Scoring Engine

- **Task Identifier:** `TASK-S2P4-05`
- **Component:** `crates/gitupload-storage/src/repositories/leaderboard.rs`
- **SRS Traceability:** `FR-SET-001`, `FR-USE-001`

#### Implementation Specification
Rank students and team members:
- Consecutive streak days multiplier.
- Bonus points for commits conforming to Conventional Commits standard.
- Deductions for blind force pushes or broken build commits.

---

### Task TASK-S2P4-06: Primer UI Analytics Dashboard Component

- **Task Identifier:** `TASK-S2P4-06`
- **Component:** `ui/src/views/AnalyticsView.tsx`, `ui/src/components/ContributionHeatmap.tsx`
- **SRS Traceability:** `FR-USE-001`

#### Implementation Specification
GitHub Primer-styled view:
- 52-column calendar grid showing 365 squares with tooltips (`"5 commits on Nov 18, 2026"`).
- Metric cards: Current Streak, Longest Streak, Total Commits This Year, On-Time Homework Rate.
- Team Leaderboard table with rank, avatar, student name, score, and badge.

---

## 3. Strict Acceptance Criteria (AC-xxx)

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-S2P4-01** | A repository with 500 historical commits | Scanning via `CommitAggregator::scan_repository()` | Aggregates dates, insertions, and deletions in < 100ms | `cargo test -p gitupload-git test_analytics_scan` |
| **AC-S2P4-02** | 365-day query requested | Calling `AnalyticsRepository::get_heatmap(365)` | Returns exactly 365 daily data points with correct intensity tiers | `cargo test -p gitupload-storage test_heatmap_query` |
| **AC-S2P4-03** | Commits made across 5 consecutive days | Calculating streak | Returns `current_streak: 5` and calculates velocity metrics correctly | `cargo test -p gitupload-storage test_streak_calculation` |
| **AC-S2P4-04** | Homework pushed 1 hour before deadline | Evaluating assignment | Flagged as `On-Time` in database with positive score attribution | `cargo test -p gitupload-storage test_ontime_eval` |
| **AC-S2P4-05** | Multiple student contributors in SQLite | Querying leaderboard | Generates sorted rankings based on streak, volume, and quality metrics | `cargo test -p gitupload-storage test_leaderboard_rankings` |
| **AC-S2P4-06** | Analytics UI view rendered | User hovers over heatmap square | Primer Tooltip displays formatted date, commit count, and repo breakdown | UI Component Test |

---

## 4. Definition of Done (DoD) Checklist for Phase 4

- [x] Relational analytics schema compiles clean with `rusqlite`.
- [x] `#![forbid(unsafe_code)]` enforced across all analytics modules.
- [x] 100% of analytics aggregation and heatmap tests pass (`cargo test -p gitupload-storage analytics`).
- [x] 365-day heatmap query executes in < 5ms.
- [x] Clippy reports zero warnings under `cargo clippy --all-targets -- -D warnings`.
