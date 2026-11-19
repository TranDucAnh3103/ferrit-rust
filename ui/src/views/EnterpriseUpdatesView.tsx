import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  ShieldLockIcon,
  SyncIcon,
  GlobeIcon,
  LockIcon,
  CheckCircleFillIcon,
  FileIcon,
  ServerIcon,
  VerifiedIcon,
} from "@primer/octicons-react";
import {
  formatIpcError,
  type CorporateProxyConfig,
  type PlatformCapabilitiesInfo,
  type UpdateChannel,
  type UpdateManifestInfo,
} from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

export const EnterpriseUpdatesView: React.FC = () => {
  const { t } = useI18n();
  const [platformInfo, setPlatformInfo] =
    useState<PlatformCapabilitiesInfo | null>(null);
  const [channel, setChannel] = useState<UpdateChannel>("stable");
  const [updateInfo, setUpdateInfo] = useState<UpdateManifestInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<string | null>(null);

  const [proxyUrl, setProxyUrl] = useState("");
  const [caCertPath, setCaCertPath] = useState("");
  const [bypassHosts, setBypassHosts] = useState("localhost,127.0.0.1,*.internal");
  const [remoteTestUrl, setRemoteTestUrl] = useState("https://github.com/org/repo.git");
  const [remoteAllowed, setRemoteAllowed] = useState<boolean | null>(null);

  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadAll = async () => {
    try {
      const [cap, ch, prx] = await Promise.all([
        invoke<PlatformCapabilitiesInfo>("enterprise_get_policy"),
        invoke<UpdateChannel>("updater_get_channel"),
        invoke<CorporateProxyConfig>("enterprise_get_proxy"),
      ]);
      setPlatformInfo(cap);
      setChannel(ch);
      setProxyUrl(prx.proxy_url || cap.policy.corporate_proxy_url || "");
      setCaCertPath(prx.custom_ca_cert_path || "");
      if (prx.bypass_hosts.length > 0) {
        setBypassHosts(prx.bypass_hosts.join(","));
      }
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  useEffect(() => {
    loadAll();
  }, []);

  const handleToggleAirgap = async () => {
    if (!platformInfo) return;
    setBusy(true);
    setErrorMsg(null);
    setNotice(null);
    try {
      const next = await invoke<PlatformCapabilitiesInfo["policy"]>(
        "enterprise_set_airgap",
        { enabled: !platformInfo.policy.airgap_enforced }
      );
      setPlatformInfo({ ...platformInfo, policy: next });
      setNotice(
        next.airgap_enforced
          ? "Strict Enterprise Air-Gap Mode activated (external internet blocked)."
          : "Enterprise Air-Gap Mode disabled."
      );
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleChannelChange = async (next: UpdateChannel) => {
    setChannel(next);
    setErrorMsg(null);
    try {
      await invoke("updater_set_channel", { channel: next });
      setNotice(`Update channel switched to ${next.toUpperCase()}.`);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const handleCheckUpdates = async () => {
    setBusy(true);
    setErrorMsg(null);
    setUpdateStatus(null);
    setUpdateInfo(null);
    try {
      const found = await invoke<UpdateManifestInfo | null>(
        "updater_check_for_updates",
        { customEndpoint: null }
      );
      if (found) {
        setUpdateInfo(found);
      } else {
        setUpdateStatus("You are running the latest verified release.");
      }
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const pickCaPem = async () => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: "X.509 PEM Certificate", extensions: ["pem", "crt", "cer"] }],
    });
    if (typeof selected === "string" && selected.trim()) {
      setCaCertPath(selected);
    }
  };

  const handleSaveProxy = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusy(true);
    setErrorMsg(null);
    setNotice(null);
    try {
      const config: CorporateProxyConfig = {
        proxy_url: proxyUrl.trim(),
        custom_ca_cert_path: caCertPath.trim() || null,
        bypass_hosts: bypassHosts
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean),
      };
      await invoke("enterprise_save_proxy", { config });
      setNotice("Corporate proxy & custom Root CA validated and saved.");
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleCheckRemoteUrl = async () => {
    try {
      const ok = await invoke<boolean>("enterprise_validate_remote_url", {
        url: remoteTestUrl,
      });
      setRemoteAllowed(ok);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <ShieldLockIcon size={20} className="text-accent" />
            {t("enterprise.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("enterprise.subtitle")}
          </p>
        </div>

        {platformInfo && (
          <span className="badge d-inline-flex flex-items-center gap-1 text-mono">
            <ServerIcon size={13} />
            {platformInfo.os_name}/{platformInfo.arch}
          </span>
        )}
      </div>

      {notice && (
        <div
          className="primer-card p-2 text-success f6 d-flex flex-items-center gap-2"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          <CheckCircleFillIcon size={15} />
          <span>{notice}</span>
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

      {/* Platform & Air-Gap Enforcement Card */}
      {platformInfo && (
        <div className="primer-card p-3 d-flex flex-column gap-3">
          <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
            <div>
              <div className="f5 text-bold d-flex flex-items-center gap-2">
                <GlobeIcon size={16} className="text-accent" />
                Strict Enterprise Air-Gap &amp; Telemetry Isolation
                {platformInfo.policy.gpo_locks_airgap && (
                  <span className="badge text-attention d-inline-flex flex-items-center gap-1">
                    <LockIcon size={12} />
                    Locked by IT GPO
                  </span>
                )}
              </div>
              <div className="f6 text-muted mt-1">
                Active Vault Backend: <strong>{platformInfo.vault_backend}</strong> • Zero-Telemetry Enforced:{" "}
                <strong className="text-success">
                  {platformInfo.policy.telemetry_disabled ? "Yes" : "No"}
                </strong>
              </div>
            </div>

            <button
              type="button"
              className={
                platformInfo.policy.airgap_enforced ? "btn-danger" : "btn-secondary"
              }
              disabled={busy || platformInfo.policy.gpo_locks_airgap}
              onClick={handleToggleAirgap}
            >
              {platformInfo.policy.airgap_enforced
                ? "Air-Gap Active (Click to Disable)"
                : "Enable Strict Air-Gap Mode"}
            </button>
          </div>

          <div className="d-flex gap-2 flex-wrap">
            <input
              type="text"
              className="form-control flex-1 text-mono f6"
              value={remoteTestUrl}
              onChange={(e) => setRemoteTestUrl(e.target.value)}
              placeholder="Test remote URL against active Air-Gap & GPO policy..."
            />
            <button
              type="button"
              className="btn btn-secondary f6"
              onClick={handleCheckRemoteUrl}
            >
              Test Remote URL Policy
            </button>
            {remoteAllowed !== null && (
              <span
                className={`badge d-inline-flex flex-items-center ${
                  remoteAllowed ? "text-success" : "text-danger"
                }`}
              >
                {remoteAllowed ? "Permitted by Policy" : "Blocked by Air-Gap / Policy"}
              </span>
            )}
          </div>
        </div>
      )}

      <div className="d-flex gap-3 flex-wrap">
        {/* Signed Auto-Updater Panel */}
        <div
          className="primer-card p-3 d-flex flex-column gap-3 flex-1"
          style={{ minWidth: 300 }}
        >
          <div className="f5 text-bold d-flex flex-items-center gap-2">
            <VerifiedIcon size={16} className="text-success" />
            Minisign Verified Auto-Updater
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              Release Distribution Channel
            </label>
            <select
              className="form-select width-full f6"
              value={channel}
              onChange={(e) => handleChannelChange(e.target.value as UpdateChannel)}
            >
              <option value="stable">Stable (Enterprise Recommended)</option>
              <option value="beta">Beta (Release Candidates)</option>
              <option value="nightly">Nightly (Bleeding Edge)</option>
            </select>
          </div>

          <button
            type="button"
            className="btn btn-primary d-inline-flex flex-items-center flex-justify-center gap-1"
            disabled={busy}
            onClick={handleCheckUpdates}
          >
            <SyncIcon size={14} />
            Check Signed Updates ({channel})
          </button>

          {updateStatus && (
            <div className="primer-card p-2 bg-canvas-inset f6 text-muted">
              {updateStatus}
            </div>
          )}

          {updateInfo && (
            <div className="primer-card p-2 bg-canvas-inset f6">
              <div className="text-bold text-success">
                Verified Update Available: v{updateInfo.version}
              </div>
              <div className="text-muted mt-1">{updateInfo.body}</div>
            </div>
          )}
        </div>

        {/* Corporate Proxy & Root CA Certificate Manager */}
        <form
          onSubmit={handleSaveProxy}
          className="primer-card p-3 d-flex flex-column gap-2 flex-1"
          style={{ minWidth: 300 }}
        >
          <div className="f5 text-bold mb-1">
            Corporate Proxy &amp; Custom Root CA
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              Forward Proxy URL
            </label>
            <input
              type="text"
              className="form-control width-full text-mono f6"
              value={proxyUrl}
              onChange={(e) => setProxyUrl(e.target.value)}
              placeholder="http://proxy.corp.internal:8080"
            />
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              Custom Enterprise Root CA (.pem)
            </label>
            <div className="d-flex gap-2">
              <input
                type="text"
                className="form-control flex-1 text-mono f6"
                value={caCertPath}
                onChange={(e) => setCaCertPath(e.target.value)}
                placeholder="Optional X.509 Root CA PEM path..."
              />
              <button
                type="button"
                className="btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
                onClick={pickCaPem}
              >
                <FileIcon size={13} />
                Browse
              </button>
            </div>
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">
              Proxy Bypass Hosts
            </label>
            <input
              type="text"
              className="form-control width-full text-mono f6"
              value={bypassHosts}
              onChange={(e) => setBypassHosts(e.target.value)}
            />
          </div>

          <button type="submit" className="btn btn-secondary mt-1" disabled={busy}>
            Validate &amp; Save Proxy Config
          </button>
        </form>
      </div>
    </div>
  );
};

export default EnterpriseUpdatesView;
