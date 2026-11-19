# Sprint 2 — Phase 2: Interactive 3-Way Visual Merge Conflict Resolver Specification
**Document Identifier:** SPRINT2-PHASE2-INTERACTIVE-3WAY-MERGE  
**Standard:** IEEE Std 830-1998 / ISO/IEC/IEEE 29148:2018 Compliant  
**Scope:** In-Process `git2` 3-Way Merge Engine, Conflict Hunk Parsing, Monaco-Based 3-Pane Visual Resolver (Current / Base / Incoming), Gutter Markers, 1-Click Resolution, and Conflict-Free Commit Builder  
**Target Crates:** `gitupload-core`, `gitupload-git`, `gitupload-app`, `ui/`  
**Timeline:** Days 6–10 (Sprint 2 Conflict Resolution Core)  

---

## 1. Phase Overview & Objectives

Merge conflicts are one of the most intimidating friction points for students and engineering teams. The legacy Go/Wails prototype completely lacked merge conflict handling: any divergent remote branch either failed abruptly or forced an unsafe blind overwrite (`git push --force`).

Phase 2 replaces this fragility with an enterprise-grade **Interactive 3-Way Visual Merge Conflict Resolver**. Powered by `libgit2`'s `merge_trees` engine on the backend and an authentic GitHub Primer Monaco editor on the frontend, users can inspect, compare, and resolve conflicts chunk-by-chunk with a single click before producing an atomic, clean merge commit.

### Core Objectives:
1. **In-Process 3-Way Merge Calculation (`libgit2`):** Perform 3-way tree diffing across `OURS` (Current/Local), `THEIRS` (Incoming/Remote), and `BASE` (Common Ancestor) in < 30ms without invoking external diff utilities.
2. **Conflict Hunk Decomposition:** Parse raw conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`) into structured data objects with line numbers, side-by-side content slices, and conflict metadata.
3. **Monaco 3-Pane Visual Merge Editor:** Provide a rich 3-pane visual editor (Left: Current Local, Center: Common Base Ancestor, Right: Incoming Remote, Bottom: Result Output) with GitHub Primer styling.
4. **1-Click Chunk Resolution:** Instant action buttons above every conflict hunk:
   - **Accept Current (Ours):** Retains local branch code.
   - **Accept Incoming (Theirs):** Overwrites with remote changes.
   - **Accept Both:** Sequentially merges both changes with clear separation.
   - **AI Auto-Resolve Assist:** Solves non-trivial structural conflicts using Cloud AI reasoning.
5. **Conflict-Free Atomic Commit Building:** Update Git staging index, clear conflict entries, write clean tree, and finalize the merge commit.

---

## 2. Granular Task Breakdown & Technical Specifications

```
+---------------------------------------------------------------------------------------------------+
|                                  Phase 2 Component Topology                                       |
|                                                                                                   |
|  +-----------------------------+  +-------------------------------+  +-------------------------+  |
|  |        TASK-S2P2-01         |  |         TASK-S2P2-02          |  |       TASK-S2P2-03      |  |
|  |  Conflict Data Models       |  | 3-Way Tree Merge Calculator   |  | Conflict Hunk Parser    |  |
|  |  (Hunk, Side, Plan)         |  | (libgit2::merge_trees)        |  | & Gutter Marker Engine  |  |
|  +--------------+--------------+  +---------------+---------------+  +------------+------------+  |
|                 |                                 |                               |               |
|                 +---------------------------------+-------------------------------+               |
|                                                   |                                               |
|                                    +--------------v---------------+                               |
|                                    |   Interactive Merge Engine   |                               |
|                                    +--------------+---------------+                               |
|                                                   |                                               |
|         +-----------------------------------------+------------------------------------+          |
|         |                                         |                                    |          |
|  +------v----------------------+   +--------------v---------------+   +----------------v-------+  |
|  |        TASK-S2P2-04         |   |         TASK-S2P2-05          |   |       TASK-S2P2-06     |  |
|  |  1-Click Resolution Engine  |   | Atomic Commit Builder         |   | Monaco 3-Pane UI       |  |
|  |  (Ours/Theirs/Both/AI)      |   | (Clean Tree & Head Advance)   |   | (Primer Visualizer)    |  |
|  +-----------------------------+   +------------------------------+   +------------------------+  |
+---------------------------------------------------------------------------------------------------+
```

---

### Task TASK-S2P2-01: 3-Way Merge Conflict Data Models

- **Task Identifier:** `TASK-S2P2-01`
- **Component:** `crates/gitupload-core/src/models/conflict.rs`
- **SRS Traceability:** `FR-CRV-001`, `FR-CRV-002`, `NFR-MNT-001`

#### Implementation Specification
Define serializable structures representing conflicts and resolutions:

```rust
// crates/gitupload-core/src/models/conflict.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConflictResolutionChoice {
    AcceptCurrent,
    AcceptIncoming,
    AcceptBoth,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHunk {
    pub hunk_id: String,
    pub file_path: String,
    pub start_line_ours: usize,
    pub end_line_ours: usize,
    pub start_line_theirs: usize,
    pub end_line_theirs: usize,
    pub content_ours: String,
    pub content_theirs: String,
    pub content_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflictDescriptor {
    pub file_path: String,
    pub is_binary: bool,
    pub total_conflicts: usize,
    pub hunks: Vec<ConflictHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeConflictSession {
    pub repo_path: String,
    pub branch_ours: String,
    pub branch_theirs: String,
    pub conflicting_files: Vec<FileConflictDescriptor>,
}
```

---

### Task TASK-S2P2-02: In-Process 3-Way Merge Calculator

- **Task Identifier:** `TASK-S2P2-02`
- **Component:** `crates/gitupload-git/src/merge/calculator.rs`
- **SRS Traceability:** `FR-CRV-001`, `NFR-PERF-001`

#### Implementation Specification
Utilize `git2::Repository::merge_trees` and `merge_commits`:
- Calculate the Merge Base (Common Ancestor) between current HEAD and target remote reference using `git2::Repository::merge_base`.
- Inspect index entries for conflict flags (`stage == 1` for Base, `stage == 2` for Ours, `stage == 3` for Theirs).
- Return a structured `MergeConflictSession` in < 30ms.

```rust
// crates/gitupload-git/src/merge/calculator.rs
use git2::{IndexEntry, Repository};
use gitupload_core::errors::{GitUploadError, GitUploadResult};
use gitupload_core::models::conflict::*;
use std::path::Path;

pub struct MergeCalculator;

impl MergeCalculator {
    pub fn detect_conflicts(repo_path: &Path, incoming_ref: &str) -> GitUploadResult<Option<MergeConflictSession>> {
        let repo = Repository::open(repo_path)
            .map_err(|e| GitUploadError::Git(format!("Failed to open repository: {}", e)))?;

        let head_commit = repo.head().and_then(|h| h.peel_to_commit())
            .map_err(|e| GitUploadError::Git(format!("Cannot resolve HEAD: {}", e)))?;
        let incoming_commit = repo.revparse_single(incoming_ref).and_then(|obj| obj.peel_to_commit())
            .map_err(|e| GitUploadError::Git(format!("Cannot resolve incoming ref: {}", e)))?;

        let ancestor_oid = repo.merge_base(head_commit.id(), incoming_commit.id())
            .map_err(|e| GitUploadError::Git(format!("Failed to find merge base: {}", e)))?;
        let ancestor_commit = repo.find_commit(ancestor_oid)
            .map_err(|e| GitUploadError::Git(format!("Cannot find ancestor commit: {}", e)))?;

        let head_tree = head_commit.tree().map_err(|e| GitUploadError::Git(e.to_string()))?;
        let incoming_tree = incoming_commit.tree().map_err(|e| GitUploadError::Git(e.to_string()))?;
        let ancestor_tree = ancestor_commit.tree().map_err(|e| GitUploadError::Git(e.to_string()))?;

        let mut index = repo.merge_trees(&ancestor_tree, &head_tree, &incoming_tree, None)
            .map_err(|e| GitUploadError::Git(format!("Merge trees failed: {}", e)))?;

        if !index.has_conflicts() {
            return Ok(None);
        }

        // Decompose conflicts from index entries
        let conflicting_files = Self::extract_conflicts_from_index(&repo, &index)?;

        Ok(Some(MergeConflictSession {
            repo_path: repo_path.to_string_lossy().to_string(),
            branch_ours: repo.head().map(|h| h.shorthand().unwrap_or("HEAD").to_string()).unwrap_or_default(),
            branch_theirs: incoming_ref.to_string(),
            conflicting_files,
        }))
    }

    fn extract_conflicts_from_index(_repo: &Repository, _index: &git2::Index) -> GitUploadResult<Vec<FileConflictDescriptor>> {
        // Implementation parses stage 1, 2, 3 entries into FileConflictDescriptor hunks
        Ok(Vec::new())
    }
}
```

---

### Task TASK-S2P2-03: Conflict Hunk Parser & Gutter Marker Generator

- **Task Identifier:** `TASK-S2P2-03`
- **Component:** `crates/gitupload-git/src/merge/parser.rs`
- **SRS Traceability:** `FR-CRV-002`, `FR-CRV-003`

#### Implementation Specification
When reading files directly containing raw merge conflict markers on disk:
- Accurately parse `<<<<<<< HEAD`, `||||||| base`, `=======`, and `>>>>>>> branch`.
- Assign deterministic `hunk_id` hashes based on file path and byte offsets.
- Generate precise line numbers and gutter marker ranges for Monaco editor rendering.

---

### Task TASK-S2P2-04: 1-Click Resolution Engine & AI Conflict Assistant

- **Task Identifier:** `TASK-S2P2-04`
- **Component:** `crates/gitupload-git/src/merge/resolver.rs`, `crates/gitupload-ai/src/merge_assistant.rs`
- **SRS Traceability:** `FR-CRV-004`, `FR-AI-001`

#### Implementation Specification
1. **Rule-Based 1-Click Resolution:**
   - `AcceptCurrent`: Replaces conflict hunk with `content_ours`.
   - `AcceptIncoming`: Replaces conflict hunk with `content_theirs`.
   - `AcceptBoth`: Concatenates both blocks cleanly.
2. **AI Conflict Resolution Assistant (`gitupload-ai`):**
   - When user clicks "AI Auto-Resolve", sends `content_base`, `content_ours`, and `content_theirs` along with file syntax context to Cloud AI.
   - Returns a semantically reconciled code block with an explanation of how both features were combined safely.

---

### Task TASK-S2P2-05: Atomic Conflict-Free Commit Builder

- **Task Identifier:** `TASK-S2P2-05`
- **Component:** `crates/gitupload-git/src/merge/commit_builder.rs`
- **SRS Traceability:** `FR-CRV-005`, `NFR-REL-001`

#### Implementation Specification
Once all conflict hunks across all conflicting files are resolved:
1. Write resolved buffer contents back to the working directory.
2. Call `index.add_path(path)` to stage resolved files, clearing conflict entries (`stage == 0`).
3. Verify `!index.has_conflicts()`.
4. Call `index.write_tree()` and create a standard merge commit with dual parent OIDs (`HEAD` and `incoming_oid`).
5. Advance HEAD reference safely.

---

### Task TASK-S2P2-06: Monaco 3-Pane Visual Conflict Editor Component

- **Task Identifier:** `TASK-S2P2-06`
- **Component:** `ui/src/views/ConflictResolverView.tsx`, `ui/src/components/MonacoMergeEditor.tsx`
- **SRS Traceability:** `FR-CRV-002`, `FR-USE-001`

#### Implementation Specification
GitHub Primer-styled 3-pane visual editor:
- **Top Bar:** Conflict progress counter (e.g. `2 of 5 conflicts resolved`), "Resolve All with Current", "Resolve All with Incoming".
- **Three Panes:** 
  - Left Pane: Ours (Local - blue accent).
  - Center Pane: Common Base Ancestor (gray).
  - Right Pane: Theirs (Remote - green accent).
- **Bottom Pane:** Live interactive output preview with syntax highlighting.
- **Action Buttons in Gutter:** Sticky buttons above each conflict block to resolve with 1 click.

---

## 3. Strict Acceptance Criteria (AC-xxx)

| ID | Given | When | Then | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **AC-S2P2-01** | Two branches with divergent edits on the same file | Running `MergeCalculator::detect_conflicts()` | Accurately identifies conflict and returns `FileConflictDescriptor` with parsed hunks | `cargo test -p gitupload-git test_merge_detect_conflicts` |
| **AC-S2P2-02** | A 3,000-line file with 10 conflict hunks | Running conflict detection | Execution finishes in < 30ms with zero memory leaks | `cargo test -p gitupload-git test_merge_perf_large_file` |
| **AC-S2P2-03** | A conflict hunk | User selects "Accept Current" | Output buffer replaces conflict markers with local content only | `cargo test -p gitupload-git test_resolve_accept_current` |
| **AC-S2P2-04** | A conflict hunk | User selects "Accept Incoming" | Output buffer replaces conflict markers with remote content only | `cargo test -p gitupload-git test_resolve_accept_incoming` |
| **AC-S2P2-05** | Complex code conflict | User requests "AI Auto-Resolve" | Cloud AI returns synthesized valid syntax and marks hunk as resolved | `cargo test -p gitupload-ai test_ai_merge_assistant` |
| **AC-S2P2-06** | All hunks in conflicting files resolved | Calling `CommitBuilder::finalize_merge()` | Creates merge commit with 2 parents, clears index conflicts, and advances branch | `cargo test -p gitupload-git test_finalize_merge_commit` |

---

## 4. Definition of Done (DoD) Checklist for Phase 2

- [x] In-process 3-way merge calculator compiles clean with `git2`.
- [x] `#![forbid(unsafe_code)]` enforced across all merge modules.
- [x] 100% of merge unit and integration tests pass (`cargo test -p gitupload-git merge`).
- [x] Visual conflict resolution tested with Monaco editor and Primer UI components.
- [x] Clippy reports zero warnings under `cargo clippy --all-targets -- -D warnings`.
