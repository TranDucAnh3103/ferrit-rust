import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  KeyIcon,
  PlusIcon,
  ShieldCheckIcon,
  CopyIcon,
  TrashIcon,
  UploadIcon,
  PlayIcon,
  SquareIcon,
  CheckIcon,
} from "@primer/octicons-react";
import {
  formatIpcError,
  type SshAgentStatusPayload,
  type SshKeyAlgorithm,
  type SshKeyDescriptor,
} from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

export const SshKeysView: React.FC = () => {
  const { t } = useI18n();
  const [keys, setKeys] = useState<SshKeyDescriptor[]>([]);
  const [agentStatus, setAgentStatus] = useState<SshAgentStatusPayload | null>(
    null
  );
  const [title, setTitle] = useState("FERRIT Desktop - Windows");
  const [algorithm, setAlgorithm] = useState<SshKeyAlgorithm>("ed25519");
  const [busy, setBusy] = useState(false);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadData = async () => {
    try {
      const [keyList, status] = await Promise.all([
        invoke<SshKeyDescriptor[]>("ssh_list_keys"),
        invoke<SshAgentStatusPayload>("ssh_agent_status"),
      ]);
      setKeys(keyList);
      setAgentStatus(status);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleGenerate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      await invoke<SshKeyDescriptor>("ssh_generate_key", {
        title: title.trim(),
        algorithm,
      });
      await loadData();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleToggleAgent = async () => {
    setBusy(true);
    setErrorMsg(null);
    try {
      const cmd = agentStatus?.is_running
        ? "ssh_agent_stop"
        : "ssh_agent_start";
      const nextStatus = await invoke<SshAgentStatusPayload>(cmd);
      setAgentStatus(nextStatus);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleCopyPublicKey = async (key: SshKeyDescriptor) => {
    try {
      await navigator.clipboard.writeText(key.public_key);
      setCopiedId(key.key_id);
      setTimeout(() => setCopiedId(null), 2000);
    } catch {
      setErrorMsg("Clipboard write denied by browser context");
    }
  };

  const handleUploadToGitHub = async (keyId: string) => {
    setBusy(true);
    setErrorMsg(null);
    try {
      await invoke("ssh_upload_key_to_github", {
        keyId,
        accountId: "default",
      });
      await loadData();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleDelete = async (keyId: string) => {
    setBusy(true);
    setErrorMsg(null);
    try {
      await invoke("ssh_delete_key", { keyId });
      await loadData();
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
            <KeyIcon size={20} className="text-accent" />
            {t("ssh.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("ssh.subtitle")}
          </p>
        </div>

        <button
          type="button"
          className={
            agentStatus?.is_running
              ? "btn-danger d-inline-flex flex-items-center gap-1"
              : "btn-primary d-inline-flex flex-items-center gap-1"
          }
          disabled={busy}
          onClick={handleToggleAgent}
        >
          {agentStatus?.is_running ? (
            <>
              <SquareIcon size={13} />
              {t("ssh.stopAgent")}
            </>
          ) : (
            <>
              <PlayIcon size={13} />
              {t("ssh.startAgent")}
            </>
          )}
        </button>
      </div>

      {agentStatus && (
        <div className="primer-card p-3 d-flex flex-justify-between flex-items-center flex-wrap gap-3">
          <div className="d-flex flex-items-center gap-2">
            <ShieldCheckIcon
              size={18}
              className={
                agentStatus.is_running ? "text-success" : "text-muted"
              }
            />
            <div>
              <div className="f5 text-bold">
                {t("ssh.agentStatus")}:{" "}
                <span
                  className={
                    agentStatus.is_running ? "text-success" : "text-muted"
                  }
                >
                  {agentStatus.is_running ? t("common.active") : t("common.inactive")}
                </span>
              </div>
              <div className="text-mono f6 text-muted">
                {agentStatus.pipe_name}
              </div>
            </div>
          </div>

          <div className="text-right">
            <div className="f6 text-muted">Identities in RAM</div>
            <div className="f4 text-bold tabular-nums">
              {agentStatus.loaded_key_count}
            </div>
          </div>
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
        <form
          onSubmit={handleGenerate}
          className="primer-card p-3 d-flex flex-column gap-2"
          style={{ flex: "1 1 280px", maxWidth: 360 }}
        >
          <div className="f5 text-bold d-flex flex-items-center gap-2 mb-1">
            <PlusIcon size={16} className="text-accent" />
            {t("ssh.generateNew")}
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">{t("ssh.keyTitle")}</label>
            <input
              type="text"
              className="form-control width-full f6"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              required
            />
          </div>

          <div>
            <label className="d-block f6 text-muted mb-1">{t("ssh.algorithm")}</label>
            <select
              className="form-select width-full f6"
              value={algorithm}
              onChange={(e) =>
                setAlgorithm(e.target.value as SshKeyAlgorithm)
              }
            >
              <option value="ed25519">Ed25519 (Fast, Modern Curve25519)</option>
              <option value="rsa4096">RSA 4096 (SHA-256 Legacy Enterprise)</option>
            </select>
          </div>

          <button type="submit" className="btn btn-primary mt-1" disabled={busy}>
            {busy ? t("common.loading") : t("ssh.generateNew")}
          </button>
        </form>

        <div className="d-flex flex-column gap-2 flex-1" style={{ minWidth: 320 }}>
          {keys.length === 0 ? (
            <div className="empty-state-card text-center p-4">
              <div className="empty-state-icon-box">
                <KeyIcon size={24} className="text-accent" />
              </div>
              <div className="f4 text-bold">{t("ssh.noKeys")}</div>
              <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 440, margin: "8px auto" }}>
                {t("ssh.subtitle")}
              </p>
            </div>
          ) : (
            keys.map((k) => (
              <div key={k.key_id} className="primer-card p-3 interactive-row">
                <div className="d-flex flex-justify-between flex-items-start gap-2">
                  <div className="min-width-0">
                    <div className="d-flex flex-items-center gap-2 flex-wrap">
                      <strong className="f5">{k.title}</strong>
                      <span className="badge text-uppercase">
                        {k.algorithm}
                      </span>
                      {k.is_registered_on_github && (
                        <span className="badge text-success">
                          GitHub
                        </span>
                      )}
                    </div>
                    <div className="text-mono f6 text-muted mt-1">
                      {k.fingerprint_sha256}
                    </div>
                  </div>

                  <div className="d-flex flex-items-center gap-1 flex-shrink-0">
                    <button
                      type="button"
                      className="btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
                      aria-label={`Copy public key ${k.title}`}
                      onClick={() => handleCopyPublicKey(k)}
                    >
                      {copiedId === k.key_id ? (
                        <CheckIcon size={13} className="text-success" />
                      ) : (
                        <CopyIcon size={13} />
                      )}
                      {copiedId === k.key_id ? t("common.copied") : t("common.copy")}
                    </button>
                    {!k.is_registered_on_github && (
                      <button
                        type="button"
                        className="btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
                        disabled={busy}
                        aria-label={`Upload ${k.title} to GitHub`}
                        onClick={() => handleUploadToGitHub(k.key_id)}
                      >
                        <UploadIcon size={13} />
                        {t("ssh.uploadGithub")}
                      </button>
                    )}
                    <button
                      type="button"
                      className="btn-octicon text-danger"
                      aria-label={`Delete SSH key ${k.title}`}
                      onClick={() => handleDelete(k.key_id)}
                    >
                      <TrashIcon size={14} />
                    </button>
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default SshKeysView;
