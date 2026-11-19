import React from "react";
import {
  CheckIcon,
  SparklesFillIcon,
  VersionsIcon,
} from "@primer/octicons-react";
import type {
  ConflictHunk,
  ConflictResolutionChoice,
  FileConflictDescriptor,
} from "../types/ipc";

interface ThreeWayMergeEditorProps {
  file: FileConflictDescriptor;
  branchOurs: string;
  branchTheirs: string;
  choices: Record<number, ConflictResolutionChoice>;
  previewText: string;
  aiBusyHunk: number | null;
  onSelectHunkChoice: (hunkIndex: number, choice: ConflictResolutionChoice) => void;
  onAskAiResolve: (hunk: ConflictHunk) => void;
  onManualOverride: (text: string) => void;
}

export const ThreeWayMergeEditor: React.FC<ThreeWayMergeEditorProps> = ({
  file,
  branchOurs,
  branchTheirs,
  choices,
  previewText,
  aiBusyHunk,
  onSelectHunkChoice,
  onAskAiResolve,
  onManualOverride,
}) => {
  return (
    <div className="d-flex flex-column gap-3">
      {/* Synchronized 3-Pane Read-Only Reference Diff */}
      <div
        className="d-grid gap-2"
        style={{ gridTemplateColumns: "repeat(auto-fit, minmax(220px, 1fr))" }}
      >
        <div className="primer-card p-2">
          <div className="f6 text-bold text-accent mb-1">
            OURS ({branchOurs})
          </div>
          <pre
            className="merge-pane-code p-2 m-0 overflow-auto"
            style={{ maxHeight: 200 }}
          >
            {file.full_ours || "(deleted on HEAD)"}
          </pre>
        </div>

        <div className="primer-card p-2">
          <div className="f6 text-bold text-muted mb-1">
            COMMON BASE (ANCESTOR)
          </div>
          <pre
            className="merge-pane-code p-2 m-0 overflow-auto"
            style={{ maxHeight: 200 }}
          >
            {file.full_base ?? "(no common ancestor)"}
          </pre>
        </div>

        <div className="primer-card p-2">
          <div className="f6 text-bold text-success mb-1">
            THEIRS ({branchTheirs})
          </div>
          <pre
            className="merge-pane-code p-2 m-0 overflow-auto"
            style={{ maxHeight: 200 }}
          >
            {file.full_theirs || "(deleted on incoming)"}
          </pre>
        </div>
      </div>

      {/* Per-Hunk Interactive 1-Click Action Cards */}
      {!file.is_binary && (
        <div className="d-flex flex-column gap-2">
          <div className="f6 text-bold text-muted d-flex flex-items-center gap-1">
            <VersionsIcon size={14} />
            Conflict Hunks (<span className="tabular-nums">{file.hunks.length}</span>)
          </div>
          {file.hunks.map((hunk) => {
            const activeChoice = choices[hunk.hunk_index];
            const kind = activeChoice?.kind;
            return (
              <div key={hunk.hunk_id} className="primer-card p-3">
                <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2 mb-2">
                  <div className="f6 text-mono text-muted tabular-nums">
                    Hunk #{hunk.hunk_index + 1} • Lines {hunk.start_line_ours}–
                    {hunk.end_line_theirs}
                  </div>
                  <div className="d-flex flex-wrap gap-2">
                    <button
                      type="button"
                      className={`btn btn-sm ${
                        kind === "accept_current"
                          ? "btn-primary"
                          : "btn-secondary"
                      }`}
                      onClick={() =>
                        onSelectHunkChoice(hunk.hunk_index, {
                          kind: "accept_current",
                        })
                      }
                    >
                      Accept Current
                    </button>
                    <button
                      type="button"
                      className={`btn btn-sm ${
                        kind === "accept_incoming"
                          ? "btn-primary"
                          : "btn-secondary"
                      }`}
                      onClick={() =>
                        onSelectHunkChoice(hunk.hunk_index, {
                          kind: "accept_incoming",
                        })
                      }
                    >
                      Accept Incoming
                    </button>
                    <button
                      type="button"
                      className={`btn btn-sm ${
                        kind === "accept_both"
                          ? "btn-primary"
                          : "btn-secondary"
                      }`}
                      onClick={() =>
                        onSelectHunkChoice(hunk.hunk_index, {
                          kind: "accept_both",
                        })
                      }
                    >
                      Accept Both
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm d-inline-flex flex-items-center gap-1"
                      disabled={aiBusyHunk === hunk.hunk_index}
                      onClick={() => onAskAiResolve(hunk)}
                    >
                      <SparklesFillIcon size={13} className="text-attention" />
                      {aiBusyHunk === hunk.hunk_index
                        ? "Synthesizing..."
                        : "AI Resolve"}
                    </button>
                  </div>
                </div>

                <div
                  className="d-grid gap-2"
                  style={{
                    gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
                  }}
                >
                  <div>
                    <div className="f6 text-accent mb-1">Current (Ours)</div>
                    <pre className="merge-pane-code p-2 m-0 overflow-auto">
                      {hunk.content_ours || "(empty)"}
                    </pre>
                  </div>
                  <div>
                    <div className="f6 text-success mb-1">Incoming (Theirs)</div>
                    <pre className="merge-pane-code p-2 m-0 overflow-auto">
                      {hunk.content_theirs || "(empty)"}
                    </pre>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Editable Merged Result Preview */}
      <div className="primer-card p-3">
        <div className="d-flex flex-justify-between flex-items-center mb-2">
          <span className="f5 text-bold d-flex flex-items-center gap-1">
            <CheckIcon size={15} className="text-success" />
            Merged Output Result ({file.file_path})
          </span>
          <span className="f6 text-muted">
            Editable directly before committing
          </span>
        </div>
        <textarea
          className="merge-pane-code width-full p-2"
          rows={10}
          value={previewText}
          onChange={(e) => onManualOverride(e.target.value)}
        />
      </div>
    </div>
  );
};
