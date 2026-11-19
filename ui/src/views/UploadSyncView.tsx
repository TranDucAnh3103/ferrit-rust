import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  FileDirectoryIcon,
  FileIcon,
  RocketIcon,
  StopIcon,
  SparklesFillIcon,
  ShieldCheckIcon,
  CopyIcon,
  LinkExternalIcon,
  GitBranchIcon,
  PlusIcon,
} from "@primer/octicons-react";
import { FolderGit2, Cpu, KeyRound, Sparkles } from "lucide-react";
import {
  AppSettings,
  InstalledIde,
  JobLogBatchPayload,
  JobProgressPayload,
  PushHistoryRecord,
  PushMode,
  PushResult,
  RepoInspection,
  SanitizedLogLine,
  formatIpcError,
} from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

interface UploadSyncViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
  settings: AppSettings;
}

export const UploadSyncView: React.FC<UploadSyncViewProps> = ({
  repoPath,
  onSelectRepoPath,
  settings,
}) => {
  const { t } = useI18n();
  const [isFileMode, setIsFileMode] = useState(false);
  const [mode, setMode] = useState<PushMode>("SESSION_EX");
  const [sessionVal, setSessionVal] = useState("1");
  const [exVal, setExVal] = useState("1");
  const [miniProjectVal, setMiniProjectVal] = useState("");
  const [customRepoName, setCustomRepoName] = useState("");
  const [useDefaultCommit, setUseDefaultCommit] = useState(true);
  const [customCommitMsg, setCustomCommitMsg] = useState("");
  const [safeSync, setSafeSync] = useState(settings.safe_sync_enabled);
  const [inspection, setInspection] = useState<RepoInspection | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState(0);
  const [stageLabel, setStageLabel] = useState("Ready");
  const [logs, setLogs] = useState<SanitizedLogLine[]>([]);
  const [lastResult, setLastResult] = useState<PushResult | null>(null);
  const [history, setHistory] = useState<PushHistoryRecord[]>([]);
  const [ides, setIdes] = useState<InstalledIde[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  useEffect(() => {
    invoke<PushHistoryRecord[]>("system_get_history", { limit: 5 })
      .then(setHistory)
      .catch(() => {});
    invoke<InstalledIde[]>("system_detect_ides")
      .then(setIdes)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!repoPath || isFileMode) {
      setInspection(null);
      return;
    }
    invoke<RepoInspection>("git_inspect_repo", { path: repoPath })
      .then(setInspection)
      .catch(() => setInspection(null));
  }, [repoPath, isFileMode]);

  useEffect(() => {
    const unlistenProg = listen<JobProgressPayload>("job-progress", (ev) => {
      setProgress(ev.payload.percent);
      setStageLabel(`${ev.payload.stage}: ${ev.payload.message}`);
    });
    const unlistenLog = listen<JobLogBatchPayload>("job-log-batch", (ev) => {
      setLogs((prev) => [...prev.slice(-300), ...ev.payload.lines]);
    });
    return () => {
      unlistenProg.then((f) => f());
      unlistenLog.then((f) => f());
    };
  }, []);

  const pickPath = async () => {
    try {
      const selected = await openDialog({
        directory: !isFileMode,
        multiple: false,
        title: isFileMode ? "Chọn file bài tập" : "Chọn thư mục bài tập",
      });
      if (typeof selected === "string" && selected) {
        onSelectRepoPath(selected);
        if (isFileMode) {
          const filename = selected.split(/[/\\]/).pop() || "";
          const match = filename.match(/SS(\d+)[_-](?:HW|EX)(\d+)/i);
          if (match) {
            setSessionVal(String(parseInt(match[1], 10)));
            setExVal(String(parseInt(match[2], 10)));
          }
        }
      }
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const generateAiCommit = async () => {
    if (!repoPath) return;
    try {
      const diff = await invoke<string>("git_get_diff", { path: repoPath });
      const msg = await invoke<string>("ai_generate_commit", { diff });
      setUseDefaultCommit(false);
      setCustomCommitMsg(msg);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const startPush = async () => {
    if (!repoPath) {
      setErrorMsg("Vui lòng chọn thư mục hoặc file trước khi Push.");
      return;
    }
    setErrorMsg(null);
    setLogs([]);
    setBusy(true);
    setProgress(5);
    setStageLabel("Starting push pipeline...");

    try {
      if (!useDefaultCommit && customCommitMsg.trim()) {
        await invoke("hooks_validate_commit", { message: customCommitMsg.trim(), maxLen: 72 });
      }
      const res = await invoke<PushResult>("git_start_push", {
        payload: {
          folder_path: repoPath,
          mode,
          session_val: sessionVal || null,
          ex_val: exVal || null,
          mini_project_val: miniProjectVal || null,
          custom_repo_name: customRepoName || null,
          use_default_commit: useDefaultCommit,
          custom_commit_msg: useDefaultCommit ? null : customCommitMsg,
          is_file: isFileMode,
          target_org: settings.github_org_name || null,
          safe_sync: safeSync,
        },
        accountId: "default",
      });
      setLastResult(res);
      setProgress(100);
      setStageLabel("Push completed!");
      const updated = await invoke<PushHistoryRecord[]>("system_get_history", { limit: 5 });
      setHistory(updated);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
      setStageLabel("Failed");
    } finally {
      setBusy(false);
    }
  };

  const cancelPush = async () => {
    await invoke("git_cancel_job").catch(() => {});
    setBusy(false);
    setStageLabel("Cancelled by user");
  };

  const copyUrl = (id: string, url: string) => {
    navigator.clipboard.writeText(url);
    setCopiedId(id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  return (
    <div className="d-flex flex-column gap-3 pb-5">
      {/* 1. Quickstart Hero Card when no repository is opened (eliminates barren/empty feeling) */}
      {!repoPath && (
        <div className="workspace-hero-card">
          <div className="d-flex flex-items-center gap-3 mb-3">
            <div className="empty-state-icon-box m-0">
              <FolderGit2 size={28} />
            </div>
            <div>
              <h2 className="f3 text-bold m-0 text-primary">{t("upload.heroTitle")}</h2>
              <p className="text-small text-muted m-0 mt-1">{t("upload.heroDesc")}</p>
            </div>
          </div>

          <div className="d-flex flex-wrap gap-3 my-3">
            <button className="btn btn-lg btn-primary" onClick={pickPath}>
              <FileDirectoryIcon size={16} />
              <span>{t("upload.openFolder")}</span>
            </button>
            <button className="btn btn-lg btn-accent" onClick={pickPath} style={{ marginLeft: 12 }}>
              <PlusIcon size={16} />
              <span>{t("upload.initRepo")}</span>
            </button>
          </div>

          <div className="d-flex flex-wrap gap-2 pt-2 border-top border-muted">
            <div className="metric-pill">
              <Cpu size={14} className="text-success" />
              <span>{t("upload.pureRustDesc")}</span>
            </div>
            <div className="metric-pill">
              <KeyRound size={14} className="text-accent" />
              <span>{t("upload.vaultSecureDesc")}</span>
            </div>
            <div className="metric-pill">
              <Sparkles size={14} className="text-warning" />
              <span>{t("upload.aiCommitDesc")}</span>
            </div>
          </div>
        </div>
      )}

      {/* 2. Main Upload & Sync Controls Panel */}
      <div className="primer-surface p-4">
        <div className="d-flex flex-items-center flex-justify-between mb-3">
          <div>
            <h2 className="f3 text-bold m-0 text-primary">
              {isFileMode ? t("upload.singleFileUpload") : t("upload.automatedUpload")}
            </h2>
            <p className="text-small text-muted m-0 mt-1">
              {t("upload.automatedUploadDesc")}
            </p>
          </div>

          {/* Mode Switcher with spaced buttons */}
          <div className="d-flex gap-2">
            <button
              type="button"
              className={`btn btn-sm ${!isFileMode ? "btn-primary" : ""}`}
              onClick={() => setIsFileMode(false)}
            >
              <FileDirectoryIcon size={14} />
              <span>{t("upload.folderMode")}</span>
            </button>
            <button
              type="button"
              className={`btn btn-sm ${isFileMode ? "btn-primary" : ""}`}
              onClick={() => setIsFileMode(true)}
              style={{ marginLeft: 8 }}
            >
              <FileIcon size={14} />
              <span>{t("upload.singleFile")}</span>
            </button>
          </div>
        </div>

        {/* Path Selection Bar */}
        <div className="d-flex flex-items-center gap-3 mb-3">
          <input
            type="text"
            className="primer-input flex-1 text-mono"
            placeholder={isFileMode ? t("upload.filePlaceholder") : t("upload.folderPlaceholder")}
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
          />
          <button className="btn" onClick={pickPath} style={{ minWidth: 140, marginLeft: 10 }}>
            <FileDirectoryIcon size={14} />
            <span>{isFileMode ? t("upload.browseFile") : t("upload.browseFolder")}</span>
          </button>
        </div>

        {/* Git Repo Status Banner */}
        {inspection?.is_git_repo && (
          <div className="flash mb-3 p-3 f6 d-flex flex-items-center flex-justify-between rounded-2">
            <div className="d-flex flex-items-center gap-2">
              <GitBranchIcon size={16} className="text-accent" />
              <span>
                <strong>{t("upload.gitRepo")}:</strong>{" "}
                <span className="Label Label--accent ml-1">{inspection.current_branch || "HEAD"}</span>{" "}
                | {t("upload.ahead")}: <strong>{inspection.ahead}</strong> | {t("upload.behind")}: <strong>{inspection.behind}</strong> | {t("upload.modified")}:{" "}
                <strong>{inspection.dirty_files.length} {t("upload.files")}</strong>
              </span>
            </div>
            <span className="text-mono text-muted f6">{inspection.remote_url || t("upload.noOrigin")}</span>
          </div>
        )}

        {/* Upload Mode Config */}
        <div className="d-flex flex-wrap gap-3 mb-3" style={{ rowGap: 16 }}>
          <div style={{ minWidth: 200, marginRight: 8 }}>
            <label className="d-block text-bold f6 mb-1">{t("upload.uploadMode")}</label>
            <select
              className="primer-input width-full"
              value={mode}
              onChange={(e) => setMode(e.target.value as PushMode)}
            >
              <option value="SESSION_EX">{t("upload.modeSessionEx")}</option>
              <option value="MINI_PROJECT">{t("upload.modeMiniProject")}</option>
              <option value="CUSTOM_REPOSITORY">{t("upload.modeCustomRepo")}</option>
            </select>
          </div>

          {(mode === "SESSION_EX" || mode === "MINI_PROJECT") && (
            <div style={{ marginRight: 8 }}>
              <label className="d-block text-bold f6 mb-1">{t("upload.session")}</label>
              <input
                type="number"
                className="primer-input"
                style={{ width: 100 }}
                value={sessionVal}
                onChange={(e) => setSessionVal(e.target.value)}
              />
            </div>
          )}

          {mode === "SESSION_EX" && (
            <div>
              <label className="d-block text-bold f6 mb-1">{t("upload.exercise")}</label>
              <input
                type="number"
                className="primer-input"
                style={{ width: 100 }}
                value={exVal}
                onChange={(e) => setExVal(e.target.value)}
              />
            </div>
          )}

          {mode === "MINI_PROJECT" && (
            <div className="flex-1" style={{ minWidth: 200 }}>
              <label className="d-block text-bold f6 mb-1">{t("upload.miniProjectName")}</label>
              <input
                type="text"
                className="primer-input width-full"
                placeholder="e.g. todo-app"
                value={miniProjectVal}
                onChange={(e) => setMiniProjectVal(e.target.value)}
              />
            </div>
          )}

          {mode === "CUSTOM_REPOSITORY" && (
            <div className="flex-1" style={{ minWidth: 200 }}>
              <label className="d-block text-bold f6 mb-1">{t("upload.repoName")}</label>
              <input
                type="text"
                className="primer-input width-full"
                placeholder="my-awesome-repo"
                value={customRepoName}
                onChange={(e) => setCustomRepoName(e.target.value)}
              />
            </div>
          )}
        </div>

        {/* Commit Options & Safe Sync Checkboxes */}
        <div className="d-flex flex-wrap flex-items-center gap-4 mb-3 p-3 bg-canvas-inset rounded-2 border border-muted">
          <label className="d-flex flex-items-center gap-2 f6 cursor-pointer" style={{ marginRight: 20 }}>
            <input
              type="checkbox"
              checked={useDefaultCommit}
              onChange={(e) => setUseDefaultCommit(e.target.checked)}
            />
            <span style={{ marginLeft: 6 }}>{t("upload.defaultCommitMsg")}</span>
          </label>
          <label className="d-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={safeSync}
              onChange={(e) => setSafeSync(e.target.checked)}
            />
            <ShieldCheckIcon size={14} className="text-success ml-1" />
            <span style={{ marginLeft: 4 }}>{t("upload.safeSyncOption")}</span>
          </label>

          {!useDefaultCommit && (
            <div className="flex-1 d-flex gap-2" style={{ minWidth: 280, marginLeft: 12 }}>
              <input
                type="text"
                className="primer-input flex-1"
                placeholder="feat(hw): add assignment implementation"
                value={customCommitMsg}
                onChange={(e) => setCustomCommitMsg(e.target.value)}
              />
              <button className="btn btn-sm" onClick={generateAiCommit} style={{ marginLeft: 8 }}>
                <SparklesFillIcon size={14} className="text-accent" />
                <span>{t("upload.aiGenerate")}</span>
              </button>
            </div>
          )}
        </div>

        {errorMsg && <div className="flash flash-error mb-3 p-3 f6 rounded-2">{errorMsg}</div>}

        {/* Action Button Toolbar with Generous Padding & Gaps */}
        <div className="d-flex flex-wrap flex-items-center gap-3">
          <button className="btn btn-lg btn-primary" onClick={startPush} disabled={busy}>
            <RocketIcon size={16} />
            <span>{busy ? t("upload.pushing") : t("upload.pushBtn")}</span>
          </button>
          <button className="btn btn-lg btn-danger" onClick={cancelPush} disabled={!busy} style={{ marginLeft: 12 }}>
            <StopIcon size={16} />
            <span>{t("common.cancel")}</span>
          </button>

          {ides.slice(0, 2).map((ide) => (
            <button
              key={ide.name}
              className="btn"
              disabled={!repoPath}
              onClick={() => invoke("system_open_in_ide", { idePath: ide.path, folderPath: repoPath })}
              style={{ marginLeft: 10 }}
            >
              <span>{t("upload.openInIde")} {ide.name}</span>
            </button>
          ))}
        </div>
      </div>

      {/* 3. Progress & 60Hz Streaming Console */}
      <div className="primer-surface p-4">
        <div className="d-flex flex-justify-between f6 mb-2">
          <span className="text-bold text-primary">{stageLabel}</span>
          <span className="text-mono tabular-nums text-muted">{progress}%</span>
        </div>
        <div className="Progress mb-3" style={{ height: 10, borderRadius: 5 }}>
          <span
            className="Progress-item color-bg-success-emphasis"
            style={{ width: `${progress}%`, transition: "width 200ms ease" }}
          />
        </div>

        <div
          className="bg-canvas-inset border border-muted rounded-2 p-3 text-mono f6 overflow-auto"
          style={{ height: 180 }}
        >
          {logs.length === 0 ? (
            <div className="text-muted">{t("upload.consoleOutput")}</div>
          ) : (
            logs.map((line, i) => (
              <div
                key={i}
                className={
                  line.level === "error"
                    ? "text-danger"
                    : line.level === "success"
                    ? "text-success"
                    : ""
                }
              >
                <span className="text-muted mr-2">[{line.timestamp}]</span>
                {line.text}
              </div>
            ))
          )}
        </div>
      </div>

      {/* 4. Recent Push History */}
      {(lastResult || history.length > 0) && (
        <div className="primer-surface p-4">
          <h3 className="f4 text-bold mb-3 text-primary">{t("upload.historyTitle")}</h3>
          <div className="d-flex flex-column gap-2">
            {history.slice(0, 4).map((rec) => (
              <div
                key={rec.id}
                className="d-flex flex-items-center flex-justify-between p-3 bg-canvas-inset rounded-2 border border-muted f6"
              >
                <div>
                  <span className="text-bold text-primary mr-3">{rec.repo_name}</span>
                  <span className="text-muted text-mono">{rec.commit_msg}</span>
                </div>
                <div className="d-flex gap-2">
                  <button
                    className="btn btn-sm"
                    onClick={() => copyUrl(rec.id, rec.repo_url)}
                  >
                    <CopyIcon size={14} />
                    <span>{copiedId === rec.id ? t("common.copied") : t("common.copy")}</span>
                  </button>
                  <button
                    className="btn btn-sm"
                    onClick={() => invoke("system_open_browser", { url: rec.repo_url })}
                  >
                    <LinkExternalIcon size={14} />
                    <span>{t("common.view")}</span>
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default UploadSyncView;
