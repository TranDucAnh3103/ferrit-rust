import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  GitBranchIcon,
  TrashIcon,
  FileDirectoryIcon,
  SyncIcon,
  ShieldLockIcon,
  CheckCircleFillIcon,
} from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import {
  formatIpcError,
  type PruneBranchesResult,
  type StaleBranchInfo,
} from "../types/ipc";

interface StaleBranchViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const StaleBranchView: React.FC<StaleBranchViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [branches, setBranches] = useState<StaleBranchInfo[]>([]);
  const [selectedBranches, setSelectedBranches] = useState<string[]>([]);
  const [defaultBranch, setDefaultBranch] = useState("main");
  const [deleteRemote, setDeleteRemote] = useState(false);
  const [busy, setBusy] = useState(false);
  const [pruneSummary, setPruneSummary] = useState<PruneBranchesResult | null>(
    null
  );
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const fetchBranches = async () => {
    if (!repoPath.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      const res = await invoke<StaleBranchInfo[]>("git_get_stale_branches", {
        repoPath,
        defaultBranch: defaultBranch.trim() || "main",
        githubOwner: null,
        githubRepo: null,
      });
      setBranches(res);
      const preselected = res
        .filter(
          (b) =>
            !b.is_protected &&
            (b.merge_status === "FullyMerged" ||
              b.merge_status === "SquashedAndMerged")
        )
        .map((b) => b.name);
      setSelectedBranches(preselected);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    if (repoPath.trim()) {
      fetchBranches();
    }
  }, [repoPath]);

  const toggleSelect = (name: string) => {
    setSelectedBranches((prev) =>
      prev.includes(name) ? prev.filter((x) => x !== name) : [...prev, name]
    );
  };

  const handlePrune = async () => {
    if (!repoPath.trim() || selectedBranches.length === 0) return;
    setBusy(true);
    setErrorMsg(null);
    setPruneSummary(null);
    try {
      const result = await invoke<PruneBranchesResult>("git_prune_branches", {
        req: {
          repo_path: repoPath,
          branch_names: selectedBranches,
          delete_remote: deleteRemote,
          prune_tracking_refs: true,
        },
      });
      setPruneSummary(result);
      await fetchBranches();
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
            <GitBranchIcon size={20} className="text-accent" />
            {t("branches.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("branches.subtitle")}
          </p>
        </div>

        <div className="d-flex flex-items-center gap-2">
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            onClick={pickFolder}
          >
            <FileDirectoryIcon size={14} />
            {t("common.browse")}
          </button>
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            disabled={busy || !repoPath.trim()}
            onClick={fetchBranches}
          >
            <SyncIcon size={14} />
            {busy ? t("common.loading") : t("branches.scanBtn")}
          </button>
          <button
            type="button"
            className="btn btn-danger d-inline-flex flex-items-center gap-1"
            disabled={busy || selectedBranches.length === 0}
            onClick={handlePrune}
          >
            <TrashIcon size={14} />
            {t("branches.pruneBtn")} (<span className="tabular-nums">{selectedBranches.length}</span>)
          </button>
        </div>
      </div>

      <div className="primer-card p-3 d-flex flex-items-end flex-wrap gap-3">
        <div className="flex-1" style={{ minWidth: 220 }}>
          <label className="d-block f6 text-muted mb-1">{t("security.targetRepo")}</label>
          <input
            type="text"
            className="form-control width-full text-mono f6"
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
            placeholder={t("upload.folderPlaceholder")}
          />
        </div>
        <div style={{ width: 150 }}>
          <label className="d-block f6 text-muted mb-1">Base Branch</label>
          <input
            type="text"
            className="form-control width-full text-mono f6"
            value={defaultBranch}
            onChange={(e) => setDefaultBranch(e.target.value)}
          />
        </div>
        <label className="d-inline-flex flex-items-center gap-2 pb-1 f6 cursor-pointer">
          <input
            type="checkbox"
            checked={deleteRemote}
            onChange={(e) => setDeleteRemote(e.target.checked)}
          />
          <span>Also delete matching branches on origin</span>
        </label>
      </div>

      {pruneSummary && (
        <div
          className="primer-card p-2 text-success f6 d-flex flex-items-center gap-2 tabular-nums"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          <CheckCircleFillIcon size={15} />
          <span>
            Pruned {pruneSummary.deleted_local_count} local branch(es) and{" "}
            {pruneSummary.deleted_remote_count} remote branch(es).
            {pruneSummary.failed_deletions.length > 0 &&
              ` (${pruneSummary.failed_deletions.length} blocked/failed)`}
          </span>
        </div>
      )}

      {errorMsg && (
        <div
          className="primer-card p-2 text-danger f6"
          style={{ borderColor: "var(--color-danger-fg)" }}
        >
          {errorMsg}
        </div>
      )}

      {branches.length === 0 ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <GitBranchIcon size={24} className="text-accent" />
          </div>
          <div className="f4 text-bold">{t("branches.title")}</div>
          <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 460, margin: "8px auto" }}>
            {t("branches.subtitle")}
          </p>
          <div className="d-flex flex-justify-center gap-2 mt-2">
            <button
              type="button"
              className="btn btn-secondary d-inline-flex flex-items-center gap-1"
              disabled={busy || !repoPath.trim()}
              onClick={fetchBranches}
            >
              <SyncIcon size={14} />
              {t("branches.scanBtn")}
            </button>
          </div>
        </div>
      ) : (
        <div className="d-flex flex-column gap-2">
          {branches.map((b) => {
            const checked = selectedBranches.includes(b.name);
            return (
              <div
                key={b.name}
                className="primer-card p-3 d-flex flex-justify-between flex-items-center gap-3 interactive-row"
              >
                <div className="d-flex flex-items-center gap-2 min-width-0">
                  <input
                    type="checkbox"
                    checked={checked}
                    disabled={b.is_protected}
                    aria-label={`Select branch ${b.name}`}
                    onChange={() => toggleSelect(b.name)}
                  />
                  <div className="min-width-0">
                    <div className="d-flex flex-items-center gap-2 flex-wrap">
                      <strong className="text-mono f5">{b.name}</strong>
                      <span
                        className={`badge ${
                          b.merge_status === "FullyMerged" ||
                          b.merge_status === "SquashedAndMerged"
                            ? "text-success"
                            : b.merge_status === "StaleInactive"
                            ? "text-attention"
                            : "text-muted"
                        }`}
                      >
                        {b.merge_status}
                      </span>
                      {b.is_protected && (
                        <span className="badge text-danger d-inline-flex flex-items-center gap-1">
                          <ShieldLockIcon size={12} />
                          Protected
                        </span>
                      )}
                      {b.associated_pr_number && (
                        <span className="badge tabular-nums">
                          PR #{b.associated_pr_number}
                        </span>
                      )}
                    </div>
                    <div className="f6 text-muted mt-1">
                      Author: {b.author_name} • SHA:{" "}
                      <code className="text-mono">
                        {b.last_commit_sha.slice(0, 8)}
                      </code>
                    </div>
                  </div>
                </div>

                <div className="f6 text-muted tabular-nums flex-shrink-0">
                  {b.days_since_last_commit} days ago
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};

export default StaleBranchView;
