import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  SyncIcon,
  PlusIcon,
  PulseIcon,
  FileDirectoryIcon,
  RepoPushIcon,
  ShieldLockIcon,
} from "@primer/octicons-react";
import { RemoteSyncCard } from "../components/RemoteSyncCard";
import { useI18n } from "../i18n/I18nContext";
import {
  formatIpcError,
  type MultiRemoteProgressPayload,
  type RemoteEndpoint,
  type RemoteHealthReport,
  type RemoteProviderType,
  type RemotePushResult,
  type SyncPolicy,
} from "../types/ipc";

interface MultiRemoteViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const MultiRemoteView: React.FC<MultiRemoteViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [endpoints, setEndpoints] = useState<RemoteEndpoint[]>([]);
  const [branch, setBranch] = useState("main");
  const [forceWithLease, setForceWithLease] = useState(false);
  const [progress, setProgress] = useState<Record<string, number>>({});
  const [overallPct, setOverallPct] = useState<number>(0);
  const [results, setResults] = useState<Record<string, RemotePushResult>>({});
  const [healthMap, setHealthMap] = useState<Record<string, RemoteHealthReport>>({});
  const [busy, setBusy] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Form inputs
  const [name, setName] = useState("gitlab");
  const [url, setUrl] = useState("");
  const [provider, setProvider] = useState<RemoteProviderType>("gitlab");
  const [syncPolicy, setSyncPolicy] = useState<SyncPolicy>("bidirectional");
  const [authAccount, setAuthAccount] = useState("default");
  const [timeoutSecs, setTimeoutSecs] = useState(30);
  const [remoteToken, setRemoteToken] = useState("");

  const loadRemotes = async (path: string) => {
    if (!path.trim()) return;
    setErrorMsg(null);
    try {
      const list = await invoke<RemoteEndpoint[]>("mirror_list_remotes", {
        repoPath: path,
      });
      setEndpoints(list);
    } catch (e) {
      setErrorMsg(formatIpcError(e));
    }
  };

  useEffect(() => {
    if (repoPath) {
      loadRemotes(repoPath);
    }
  }, [repoPath]);

  useEffect(() => {
    const unlistenPromise = listen<MultiRemoteProgressPayload>(
      "multi_remote_progress",
      (event) => {
        setProgress(event.payload.remotes);
        setOverallPct(event.payload.overall_percentage);
      }
    );
    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, []);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const handleSaveRemote = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!repoPath.trim() || !name.trim() || !url.trim()) return;
    setErrorMsg(null);
    const endpoint: RemoteEndpoint = {
      name: name.trim(),
      url: url.trim(),
      provider,
      sync_policy: syncPolicy,
      auth_account: authAccount.trim(),
      is_enabled: true,
      timeout_seconds: Math.max(1, Number(timeoutSecs) || 30),
    };
    try {
      await invoke("mirror_upsert_remote", {
        repoPath,
        remote: endpoint,
        token: remoteToken.trim() || null,
      });
      setRemoteToken("");
      setUrl("");
      await loadRemotes(repoPath);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const handleToggleEnabled = async (ep: RemoteEndpoint) => {
    try {
      await invoke("mirror_upsert_remote", {
        repoPath,
        remote: { ...ep, is_enabled: !ep.is_enabled },
        token: null,
      });
      await loadRemotes(repoPath);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const handleDelete = async (remoteName: string) => {
    try {
      await invoke("mirror_delete_remote", { repoPath, remoteName });
      await loadRemotes(repoPath);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const handleProbeAll = async () => {
    if (!repoPath.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      const reports = await invoke<RemoteHealthReport[]>(
        "mirror_probe_remotes",
        { repoPath }
      );
      const map: Record<string, RemoteHealthReport> = {};
      for (const r of reports) map[r.remote_name] = r;
      setHealthMap(map);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handlePushAll = async () => {
    if (!repoPath.trim() || !branch.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    setResults({});
    setProgress({});
    setOverallPct(0);
    try {
      const pushResults = await invoke<RemotePushResult[]>("mirror_push_all", {
        repoPath,
        branch,
        forceWithLease,
      });
      const map: Record<string, RemotePushResult> = {};
      for (const r of pushResults) map[r.remote_name] = r;
      setResults(map);
      setOverallPct(100);
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
            <SyncIcon size={20} className="text-accent" />
            {t("mirror.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("mirror.subtitle")}
          </p>
        </div>

        <div className="d-flex flex-items-center gap-3">
          <button
            type="button"
            className="btn"
            onClick={pickFolder}
          >
            <FileDirectoryIcon size={14} />
            <span>{t("mirror.selectRepo")}</span>
          </button>
          <button
            type="button"
            className="btn"
            disabled={busy || endpoints.length === 0}
            onClick={handleProbeAll}
            style={{ marginLeft: 10 }}
          >
            <PulseIcon size={14} />
            <span>{t("mirror.testConnections")}</span>
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy || endpoints.length === 0}
            onClick={handlePushAll}
            style={{ marginLeft: 10 }}
          >
            <RepoPushIcon size={14} />
            <span>{busy ? t("mirror.mirroring") : t("mirror.pushAll")}</span>
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

      <div className="primer-card p-3 d-flex flex-items-center flex-wrap gap-3">
        <div className="flex-1" style={{ minWidth: 240 }}>
          <label className="d-block f6 text-muted mb-1">{t("mirror.activeRepo")}</label>
          <input
            type="text"
            className="primer-input width-full text-mono f6"
            placeholder={t("mirror.activeRepoPlaceholder")}
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
          />
        </div>
        <div style={{ width: 170, marginLeft: 8 }}>
          <label className="d-block f6 text-muted mb-1">{t("mirror.targetBranch")}</label>
          <input
            type="text"
            className="primer-input width-full text-mono f6"
            value={branch}
            onChange={(e) => setBranch(e.target.value)}
          />
        </div>
        <div className="d-flex flex-items-center gap-2" style={{ paddingTop: 22, marginLeft: 12 }}>
          <label className="d-inline-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={forceWithLease}
              onChange={(e) => setForceWithLease(e.target.checked)}
            />
            <span style={{ marginLeft: 4 }}>{t("mirror.safeForceWithLease")}</span>
          </label>
        </div>
        {overallPct > 0 && (
          <div className="ml-auto text-right">
            <div className="f6 text-muted">{t("mirror.aggregateProgress")}</div>
            <div className="f4 text-bold tabular-nums text-success">
              {overallPct}%
            </div>
          </div>
        )}
      </div>

      <div className="d-flex gap-3 flex-wrap">
        <form
          onSubmit={handleSaveRemote}
          className="primer-card p-3 d-flex flex-column gap-2"
          style={{ flex: "1 1 300px", maxWidth: 380 }}
        >
          <div className="f5 text-bold d-flex flex-items-center gap-2 mb-1">
            <PlusIcon size={16} className="text-accent" />
            {t("mirror.addRemote")}
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">{t("mirror.remoteAlias")}</label>
            <input
              type="text"
              className="form-control width-full f6 text-mono"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("mirror.remoteAliasPlaceholder")}
              required
            />
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              {t("mirror.cleanUrl")}
            </label>
            <input
              type="text"
              className="form-control width-full f6 text-mono"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder={t("mirror.cleanUrlPlaceholder")}
              required
            />
          </div>

          <div className="d-flex gap-2">
            <div className="flex-1">
              <label className="d-block f6 text-muted mb-1">{t("mirror.provider")}</label>
              <select
                className="form-select width-full f6"
                value={provider}
                onChange={(e) =>
                  setProvider(e.target.value as RemoteProviderType)
                }
              >
                <option value="github">GitHub</option>
                <option value="gitlab">GitLab</option>
                <option value="bitbucket">Bitbucket</option>
                <option value="gitea">Gitea</option>
                <option value="custom">Custom / Bare</option>
              </select>
            </div>
            <div className="flex-1">
              <label className="d-block f6 text-muted mb-1">{t("mirror.syncPolicy")}</label>
              <select
                className="form-select width-full f6"
                value={syncPolicy}
                onChange={(e) => setSyncPolicy(e.target.value as SyncPolicy)}
              >
                <option value="bidirectional">{t("mirror.policyBidirectional")}</option>
                <option value="push_only">{t("mirror.policyPushOnly")}</option>
                <option value="backup_mirror">{t("mirror.policyBackupMirror")}</option>
              </select>
            </div>
          </div>

          <div className="d-flex gap-2">
            <div className="flex-1">
              <label className="d-block f6 text-muted mb-1">{t("mirror.vaultAccount")}</label>
              <input
                type="text"
                className="form-control width-full f6"
                value={authAccount}
                onChange={(e) => setAuthAccount(e.target.value)}
                placeholder="default"
              />
            </div>
            <div style={{ width: 96 }}>
              <label className="d-block f6 text-muted mb-1">{t("mirror.timeoutSec")}</label>
              <input
                type="number"
                min={1}
                max={300}
                className="form-control width-full f6 tabular-nums"
                value={timeoutSecs}
                onChange={(e) => setTimeoutSecs(Number(e.target.value))}
              />
            </div>
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1 d-flex flex-items-center gap-1">
              <ShieldLockIcon size={12} />
              {t("mirror.remoteToken")}
            </label>
            <input
              type="password"
              className="form-control width-full f6 text-mono"
              value={remoteToken}
              onChange={(e) => setRemoteToken(e.target.value)}
              placeholder={t("mirror.remoteTokenPlaceholder")}
            />
          </div>

          <button type="submit" className="btn btn-primary mt-2">
            {t("mirror.saveEndpoint")}
          </button>
        </form>

        <div className="d-flex flex-column gap-2 flex-1" style={{ minWidth: 320 }}>
          {endpoints.length === 0 ? (
            <div className="empty-state-card text-center p-4">
              <div className="empty-state-icon-box">
                <SyncIcon size={24} className="text-accent" />
              </div>
              <div className="f4 text-bold">{t("mirror.noRemotes")}</div>
              <p className="f6 text-muted mt-2 mb-0" style={{ maxWidth: 420, margin: "8px auto" }}>
                {t("mirror.noRemotesDesc")}
              </p>
            </div>
          ) : (
            endpoints.map((ep) => (
              <RemoteSyncCard
                key={ep.name}
                endpoint={ep}
                progressPercent={progress[ep.name]}
                lastResult={results[ep.name]}
                health={healthMap[ep.name]}
                onToggleEnabled={handleToggleEnabled}
                onDelete={handleDelete}
              />
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default MultiRemoteView;
