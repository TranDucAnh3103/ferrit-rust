import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  GitMergeIcon,
  FileDirectoryIcon,
  CheckCircleFillIcon,
  FileCodeIcon,
  SearchIcon,
} from "@primer/octicons-react";
import { ThreeWayMergeEditor } from "../components/ThreeWayMergeEditor";
import { useI18n } from "../i18n/I18nContext";
import {
  formatIpcError,
  type ConflictHunk,
  type ConflictResolutionChoice,
  type FileResolutionRequest,
  type MergeConflictSession,
} from "../types/ipc";

interface ConflictResolverViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const ConflictResolverView: React.FC<ConflictResolverViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [incomingRef, setIncomingRef] = useState("origin/main");
  const [session, setSession] = useState<MergeConflictSession | null>(null);
  const [activeFileIdx, setActiveFileIdx] = useState(0);
  const [choicesByFile, setChoicesByFile] = useState<
    Record<string, Record<number, ConflictResolutionChoice>>
  >({});
  const [previewByFile, setPreviewByFile] = useState<Record<string, string>>({});
  const [manualOverrideByFile, setManualOverrideByFile] = useState<
    Record<string, string | null>
  >({});
  const [commitMsg, setCommitMsg] = useState("");
  const [aiBusyHunk, setAiBusyHunk] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [committedOid, setCommittedOid] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const handleDetect = async () => {
    if (!repoPath.trim() || !incomingRef.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    setCommittedOid(null);
    try {
      const detected = await invoke<MergeConflictSession>(
        "merge_detect_conflicts",
        { repoPath, incomingRef }
      );
      setSession(detected);
      setActiveFileIdx(0);

      const nextChoices: Record<
        string,
        Record<number, ConflictResolutionChoice>
      > = {};
      const nextPreviews: Record<string, string> = {};
      const nextManual: Record<string, string | null> = {};
      for (const file of detected.conflicting_files) {
        nextChoices[file.file_path] = {};
        nextPreviews[file.file_path] = file.raw_conflicted_text || file.full_ours;
        nextManual[file.file_path] = null;
      }
      setChoicesByFile(nextChoices);
      setPreviewByFile(nextPreviews);
      setManualOverrideByFile(nextManual);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const refreshPreview = async (
    filePath: string,
    rawConflictedText: string,
    hunks: ConflictHunk[],
    fileChoices: Record<number, ConflictResolutionChoice>
  ) => {
    const allChosen = hunks.every((h) => fileChoices[h.hunk_index] !== undefined);
    if (!allChosen) return;
    try {
      const req: FileResolutionRequest = {
        file_path: filePath,
        chunk_resolutions: Object.entries(fileChoices).map(([idx, c]) => [
          Number(idx),
          c,
        ]),
        custom_content: null,
      };
      const merged = await invoke<string>("merge_preview_resolution", {
        rawConflictedText,
        hunks,
        request: req,
      });
      setPreviewByFile((prev) => ({ ...prev, [filePath]: merged }));
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const activeFile = session?.conflicting_files[activeFileIdx] ?? null;

  const handleSelectHunkChoice = (
    hunkIndex: number,
    choice: ConflictResolutionChoice
  ) => {
    if (!activeFile) return;
    const fp = activeFile.file_path;
    const updated = {
      ...(choicesByFile[fp] ?? {}),
      [hunkIndex]: choice,
    };
    setChoicesByFile((prev) => ({ ...prev, [fp]: updated }));
    setManualOverrideByFile((prev) => ({ ...prev, [fp]: null }));
    refreshPreview(fp, activeFile.raw_conflicted_text, activeFile.hunks, updated);
  };

  const handleAskAiResolve = async (hunk: ConflictHunk) => {
    setAiBusyHunk(hunk.hunk_index);
    setErrorMsg(null);
    try {
      const proposed = await invoke<string>("merge_ai_auto_resolve", { hunk });
      handleSelectHunkChoice(hunk.hunk_index, {
        kind: "custom",
        content: proposed,
      });
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setAiBusyHunk(null);
    }
  };

  const allFilesResolved =
    session !== null &&
    session.conflicting_files.length > 0 &&
    session.conflicting_files.every((f) => {
      if (manualOverrideByFile[f.file_path] !== null && manualOverrideByFile[f.file_path] !== undefined) {
        return true;
      }
      const chosen = choicesByFile[f.file_path] ?? {};
      return f.hunks.every((h) => chosen[h.hunk_index] !== undefined);
    });

  const handleFinalizeCommit = async () => {
    if (!session) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      const resolutions: FileResolutionRequest[] = session.conflicting_files.map(
        (f) => ({
          file_path: f.file_path,
          chunk_resolutions: Object.entries(
            choicesByFile[f.file_path] ?? {}
          ).map(([idx, choice]) => [Number(idx), choice]),
          custom_content: manualOverrideByFile[f.file_path] ?? null,
        })
      );
      const oid = await invoke<string>("merge_finalize_commit", {
        repoPath: session.repo_path,
        incomingRef: session.branch_theirs,
        resolutions,
        commitMessage: commitMsg.trim() || null,
      });
      setCommittedOid(oid);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <GitMergeIcon size={20} className="text-accent" />
            {t("merge.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("merge.subtitle")}
          </p>
        </div>

        <button
          type="button"
          className="btn"
          onClick={pickFolder}
        >
          <FileDirectoryIcon size={14} />
          <span>{t("merge.selectRepo")}</span>
        </button>
      </div>

      <div className="primer-card p-3 d-flex flex-items-end flex-wrap gap-2">
        <div className="flex-1" style={{ minWidth: 240 }}>
          <label className="d-block f6 text-muted mb-1">{t("merge.repoPath")}</label>
          <input
            type="text"
            className="form-control width-full text-mono f6"
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
            placeholder={t("merge.repoPathPlaceholder")}
          />
        </div>
        <div style={{ width: 220 }}>
          <label className="d-block f6 text-muted mb-1">
            {t("merge.incomingBranch")}
          </label>
          <input
            type="text"
            className="form-control width-full text-mono f6"
            value={incomingRef}
            onChange={(e) => setIncomingRef(e.target.value)}
            placeholder={t("merge.incomingPlaceholder")}
          />
        </div>
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy || !repoPath.trim()}
          onClick={handleDetect}
        >
          <SearchIcon size={14} />
          <span>{busy ? t("merge.analyzing") : t("merge.inspectMerge")}</span>
        </button>
      </div>

      {errorMsg && (
        <div
          className="primer-card p-2 text-danger f6"
          style={{ borderColor: "var(--color-danger-fg)" }}
        >
          {errorMsg}
        </div>
      )}

      {committedOid && (
        <div
          className="primer-card p-3 d-flex flex-items-center gap-2 text-success"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          <CheckCircleFillIcon size={16} />
          <span>
            {t("merge.commitFinalized")}{" "}
            <code className="text-mono">{committedOid.slice(0, 10)}</code>
          </span>
        </div>
      )}

      {!session ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <GitMergeIcon size={24} className="text-accent" />
          </div>
          <div className="f4 text-bold">{t("merge.readyInspection")}</div>
          <p className="f6 text-muted mt-2 mb-0" style={{ maxWidth: 440, margin: "8px auto" }}>
            {t("merge.readyInspectionDesc")}
          </p>
        </div>
      ) : session.conflicting_files.length === 0 ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <CheckCircleFillIcon size={24} className="text-success" />
          </div>
          <div className="f4 text-bold">{t("merge.zeroConflicts")}</div>
          <p className="f6 text-muted mt-2 mb-0" style={{ maxWidth: 440, margin: "8px auto" }}>
            <code>{session.branch_ours}</code> {t("common.or") === "hoặc" ? "và" : "and"}{" "}
            <code>{session.branch_theirs}</code> {t("merge.zeroConflictsDesc")}
          </p>
        </div>
      ) : (
        <div className="d-flex flex-column gap-3">
          <div className="d-flex gap-2 flex-wrap">
            {session.conflicting_files.map((file, idx) => (
              <button
                key={file.file_path}
                type="button"
                className={
                  idx === activeFileIdx
                    ? "btn btn-primary f6 d-inline-flex flex-items-center gap-1"
                    : "btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
                }
                onClick={() => setActiveFileIdx(idx)}
              >
                <FileCodeIcon size={13} />
                {file.file_path}
                <span className="badge tabular-nums ml-1">
                  {file.total_conflicts}
                </span>
              </button>
            ))}
          </div>

          {activeFile && (
            <ThreeWayMergeEditor
              file={activeFile}
              branchOurs={session.branch_ours}
              branchTheirs={session.branch_theirs}
              choices={choicesByFile[activeFile.file_path] ?? {}}
              previewText={previewByFile[activeFile.file_path] ?? ""}
              aiBusyHunk={aiBusyHunk}
              onSelectHunkChoice={handleSelectHunkChoice}
              onAskAiResolve={handleAskAiResolve}
              onManualOverride={(text) => {
                setPreviewByFile((prev) => ({
                  ...prev,
                  [activeFile.file_path]: text,
                }));
                setManualOverrideByFile((prev) => ({
                  ...prev,
                  [activeFile.file_path]: text,
                }));
              }}
            />
          )}

          <div className="primer-card p-3 d-flex flex-items-center flex-wrap gap-2">
            <input
              type="text"
              className="form-control flex-1 f6 text-mono"
              placeholder={`chore(merge): merge '${session.branch_theirs}' into '${session.branch_ours}'`}
              value={commitMsg}
              onChange={(e) => setCommitMsg(e.target.value)}
            />
            <button
              type="button"
              className="btn btn-primary"
              disabled={busy || !allFilesResolved}
              onClick={handleFinalizeCommit}
            >
              <CheckCircleFillIcon size={14} />
              <span>{t("merge.commitBtn")}</span>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default ConflictResolverView;
