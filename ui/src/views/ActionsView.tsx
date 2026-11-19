import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  PlayIcon,
  SyncIcon,
  XCircleIcon,
  LinkExternalIcon,
  GitBranchIcon,
  CheckCircleFillIcon,
  ClockIcon,
} from "@primer/octicons-react";
import { AppSettings, WorkflowRun, formatIpcError } from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

export const ActionsView: React.FC<{ settings: AppSettings }> = ({ settings }) => {
  const { t } = useI18n();
  const [owner, setOwner] = useState(settings.github_org_name || settings.github_username);
  const [repo, setRepo] = useState("");
  const [runs, setRuns] = useState<WorkflowRun[]>([]);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadRuns = async () => {
    if (!owner || !repo) return;
    setErrorMsg(null);
    try {
      const list = await invoke<WorkflowRun[]>("actions_list_runs", {
        owner,
        repo,
        accountId: "default",
      });
      setRuns(list);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const rerunFailed = async (runId: number) => {
    try {
      await invoke("actions_rerun_failed", { owner, repo, runId, accountId: "default" });
      await loadRuns();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const cancelRun = async (runId: number) => {
    try {
      await invoke("actions_cancel_run", { owner, repo, runId, accountId: "default" });
      await loadRuns();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="primer-surface p-3 d-flex gap-2">
        <input className="primer-input" placeholder={t("pulls.owner")} value={owner} onChange={(e) => setOwner(e.target.value)} />
        <input className="primer-input flex-1" placeholder={t("pulls.repo")} value={repo} onChange={(e) => setRepo(e.target.value)} />
        <button className="btn btn-primary" onClick={loadRuns}>
          <PlayIcon size={14} className="mr-1" /> {t("actions.fetchRuns")}
        </button>
      </div>

      {errorMsg && <div className="flash flash-error p-2 f6">{errorMsg}</div>}

      <div className="primer-surface p-3">
        <h3 className="f5 text-bold mb-2">{t("actions.workflowRuns")} ({runs.length})</h3>
        {runs.length === 0 ? (
          <div className="empty-state-card text-center p-4">
            <div className="empty-state-icon-box">
              <PlayIcon size={24} className="text-accent" />
            </div>
            <div className="f4 text-bold">{t("actions.noRuns")}</div>
            <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 440, margin: "8px auto" }}>
              {t("actions.subtitle")}
            </p>
          </div>
        ) : (
          runs.map((run) => (
            <div key={run.id} className="interactive-row d-flex flex-items-center flex-justify-between p-2 border-bottom border-muted">
              <div>
                <div className="text-bold d-flex flex-items-center gap-2">
                  {run.conclusion === "success" ? (
                    <CheckCircleFillIcon size={14} className="text-success" />
                  ) : run.conclusion === "failure" ? (
                    <XCircleIcon size={14} className="text-danger" />
                  ) : (
                    <ClockIcon size={14} className="text-accent" />
                  )}
                  <span>{run.name}</span>
                  <span
                    className={`Label ${
                      run.conclusion === "success"
                        ? "Label--success"
                        : run.conclusion === "failure"
                        ? "Label--danger"
                        : "Label--accent"
                    }`}
                  >
                    {run.conclusion || run.status}
                  </span>
                </div>
                <div className="text-small text-muted text-mono d-flex flex-items-center gap-2 mt-1">
                  <span className="d-inline-flex flex-items-center gap-1">
                    <GitBranchIcon size={12} /> {run.head_branch}
                  </span>
                  <span className="tabular-nums">{run.head_sha.slice(0, 7)}</span>
                  <span>event: {run.event}</span>
                </div>
              </div>
              <div className="d-flex gap-2">
                {run.conclusion === "failure" && (
                  <button className="btn btn-sm btn-secondary d-inline-flex flex-items-center gap-1" onClick={() => rerunFailed(run.id)}>
                    <SyncIcon size={12} /> {t("actions.rerun")}
                  </button>
                )}
                {run.status !== "completed" && (
                  <button className="btn btn-sm btn-danger d-inline-flex flex-items-center gap-1" onClick={() => cancelRun(run.id)}>
                    <XCircleIcon size={12} /> {t("actions.cancel")}
                  </button>
                )}
                <button
                  className="btn btn-sm btn-secondary"
                  aria-label={`Open run ${run.name} in browser`}
                  title="Open run in browser"
                  onClick={() => invoke("system_open_browser", { url: run.html_url })}
                >
                  <LinkExternalIcon size={12} />
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default ActionsView;
