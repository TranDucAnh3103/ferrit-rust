import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  ClockIcon,
  CheckCircleFillIcon,
  ZapIcon,
  ArchiveIcon,
  SyncIcon,
  FileDirectoryIcon,
} from "@primer/octicons-react";
import {
  formatIpcError,
  type AutoSyncOutcome,
  type PowerState,
  type SyncScheduleConfig,
} from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

interface SyncScheduleSettingsViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const SyncScheduleSettingsView: React.FC<
  SyncScheduleSettingsViewProps
> = ({ repoPath, onSelectRepoPath }) => {
  const { t } = useI18n();
  const [config, setConfig] = useState<SyncScheduleConfig>({
    enabled: true,
    interval_minutes: 30,
    allow_on_battery: false,
    backup_bundles_enabled: true,
  });
  const [powerState, setPowerState] = useState<PowerState>("ac_connected");
  const [savedNotice, setSavedNotice] = useState(false);
  const [bundlePath, setBundlePath] = useState<string | null>(null);
  const [syncOutcome, setSyncOutcome] = useState<AutoSyncOutcome | null>(null);
  const [busy, setBusy] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      invoke<SyncScheduleConfig>("sync_get_config"),
      invoke<PowerState>("sync_get_power_state"),
    ])
      .then(([cfg, pwr]) => {
        setConfig(cfg);
        setPowerState(pwr);
      })
      .catch(() => {});
  }, []);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const handleSave = async () => {
    setBusy(true);
    setErrorMsg(null);
    setSavedNotice(false);
    try {
      await invoke("sync_save_config", { config });
      const latestPower = await invoke<PowerState>("sync_get_power_state");
      setPowerState(latestPower);
      setSavedNotice(true);
      setTimeout(() => setSavedNotice(false), 3000);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRunSyncNow = async () => {
    if (!repoPath.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    setSyncOutcome(null);
    try {
      const outcome = await invoke<AutoSyncOutcome>("sync_run_now", {
        repoPath,
        remoteName: "origin",
        branchName: "main",
      });
      setSyncOutcome(outcome);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleCreateBundle = async () => {
    if (!repoPath.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    setBundlePath(null);
    try {
      const created = await invoke<string>("sync_create_bundle", {
        repoPath,
        repoName: null,
      });
      setBundlePath(created);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const renderPowerLabel = (p: PowerState) => {
    if (p === "ac_connected") return "AC Power Connected (Full Speed)";
    if (p === "battery_normal") return "On Battery (Normal)";
    return "Low Battery / Battery Saver (Auto-Paused)";
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <ClockIcon size={20} className="text-accent" />
            {t("schedule.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("schedule.subtitle")}
          </p>
        </div>

        <span className="badge d-inline-flex flex-items-center gap-1">
          <ZapIcon
            size={13}
            className={
              powerState === "battery_low_or_saver"
                ? "text-danger"
                : "text-success"
            }
          />
          {renderPowerLabel(powerState)}
        </span>
      </div>

      {savedNotice && (
        <div
          className="primer-card p-2 text-success f6 d-flex flex-items-center gap-2"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          <CheckCircleFillIcon size={15} />
          <span>{t("schedule.savedSuccess")}</span>
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

      <div className="d-flex gap-3 flex-wrap">
        <div
          className="primer-card p-3 d-flex flex-column gap-3 flex-1"
          style={{ minWidth: 300 }}
        >
          <div className="f5 text-bold">{t("schedule.title")}</div>

          <label className="d-inline-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={config.enabled}
              onChange={(e) =>
                setConfig({ ...config, enabled: e.target.checked })
              }
            />
            <span>{t("schedule.enableAutoSync")}</span>
          </label>

          <div>
            <label className="d-block f6 text-muted mb-1">
              {t("schedule.interval")}
            </label>
            <select
              className="form-select width-full f6"
              value={config.interval_minutes}
              disabled={!config.enabled}
              onChange={(e) =>
                setConfig({
                  ...config,
                  interval_minutes: Number(e.target.value),
                })
              }
            >
              <option value={15}>15m</option>
              <option value={30}>30m</option>
              <option value={60}>1h</option>
              <option value={120}>2h</option>
            </select>
          </div>

          <label className="d-inline-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={config.allow_on_battery}
              disabled={!config.enabled}
              onChange={(e) =>
                setConfig({ ...config, allow_on_battery: e.target.checked })
              }
            />
            <span>
              {t("schedule.allowBattery")}
            </span>
          </label>

          <label className="d-inline-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={config.backup_bundles_enabled}
              disabled={!config.enabled}
              onChange={(e) =>
                setConfig({
                  ...config,
                  backup_bundles_enabled: e.target.checked,
                })
              }
            />
            <span>{t("schedule.backupBundles")}</span>
          </label>

          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={handleSave}
          >
            {t("schedule.saveConfig")}
          </button>
        </div>

        <div
          className="primer-card p-3 d-flex flex-column gap-3 flex-1"
          style={{ minWidth: 300 }}
        >
          <div className="f5 text-bold">Manual Sync &amp; Cold Backup</div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              Target Repository
            </label>
            <div className="d-flex gap-2">
              <input
                type="text"
                className="form-control flex-1 text-mono f6"
                value={repoPath}
                onChange={(e) => onSelectRepoPath(e.target.value)}
                placeholder="Select local repository..."
              />
              <button
                type="button"
                className="btn btn-secondary d-inline-flex flex-items-center gap-1"
                onClick={pickFolder}
              >
                <FileDirectoryIcon size={14} />
                Browse
              </button>
            </div>
          </div>

          <div className="d-flex gap-2 flex-wrap">
            <button
              type="button"
              className="btn btn-secondary d-inline-flex flex-items-center gap-1"
              disabled={busy || !repoPath.trim()}
              onClick={handleRunSyncNow}
            >
              <SyncIcon size={14} />
              Fetch &amp; Fast-Forward Now
            </button>
            <button
              type="button"
              className="btn btn-secondary d-inline-flex flex-items-center gap-1"
              disabled={busy || !repoPath.trim()}
              onClick={handleCreateBundle}
            >
              <ArchiveIcon size={14} />
              Create .bundle Archive
            </button>
          </div>

          {syncOutcome && (
            <div className="primer-card p-2 bg-canvas-inset f6">
              {syncOutcome.kind === "up_to_date" &&
                "Local branch is already up to date with origin."}
              {syncOutcome.kind === "fast_forwarded" &&
                `Safely fast-forwarded ${syncOutcome.commits_pulled} commit(s).`}
              {syncOutcome.kind === "diverged" &&
                `Diverged state detected (ahead ${syncOutcome.ahead}, behind ${syncOutcome.behind}). Open 3-Way Merge Solver to reconcile.`}
            </div>
          )}

          {bundlePath && (
            <div className="primer-card p-2 bg-canvas-inset f6 text-success">
              Backup archive created:{" "}
              <code className="text-mono">{bundlePath}</code>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default SyncScheduleSettingsView;
