import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  ShieldCheckIcon,
  AlertIcon,
  CheckCircleFillIcon,
  PlayIcon,
  FileDirectoryIcon,
  SparklesFillIcon,
} from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import {
  formatIpcError,
  type SecurityAuditReport,
  type VulnerabilityFinding,
} from "../types/ipc";

interface SecurityScannerViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const SecurityScannerView: React.FC<SecurityScannerViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [report, setReport] = useState<SecurityAuditReport | null>(null);
  const [useCloudAi, setUseCloudAi] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [patchStatus, setPatchStatus] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const runScan = async () => {
    if (!repoPath.trim()) return;
    setScanning(true);
    setPatchStatus(null);
    setErrorMsg(null);
    try {
      const res = await invoke<SecurityAuditReport>("security_run_audit", {
        repoPath,
        useCloudAi,
      });
      setReport(res);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setScanning(false);
    }
  };

  const applyPatch = async (finding: VulnerabilityFinding) => {
    if (!finding.suggested_fix) return;
    setErrorMsg(null);
    try {
      await invoke("security_apply_patch", {
        repoPath,
        filePath: finding.file_path,
        proposal: finding.suggested_fix,
      });
      setPatchStatus(`${t("security.patchApplied")} ${finding.file_path}`);
      await runScan();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const severityClass = (sev: VulnerabilityFinding["severity"]) => {
    if (sev === "Critical" || sev === "High") return "text-danger";
    if (sev === "Medium") return "text-attention";
    return "text-muted";
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <ShieldCheckIcon size={20} className="text-accent" />
            {t("security.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("security.subtitle")}
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
            className="btn btn-primary d-inline-flex flex-items-center gap-1"
            disabled={scanning || !repoPath.trim()}
            onClick={runScan}
          >
            <PlayIcon size={14} />
            {scanning ? t("common.loading") : t("security.runScan")}
          </button>
        </div>
      </div>

      <div className="primer-card p-3 d-flex flex-items-center flex-wrap gap-3">
        <div className="flex-1" style={{ minWidth: 240 }}>
          <label className="d-block f6 text-muted mb-1">{t("security.targetRepo")}</label>
          <input
            type="text"
            className="form-control width-full text-mono f6"
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
            placeholder={t("security.targetRepoPlaceholder")}
          />
        </div>
        <label className="d-inline-flex flex-items-center gap-2 pt-3 f6 cursor-pointer">
          <input
            type="checkbox"
            checked={useCloudAi}
            onChange={(e) => setUseCloudAi(e.target.checked)}
          />
          <span>{t("security.enableCloudAi")}</span>
        </label>
      </div>

      {patchStatus && (
        <div
          className="primer-card p-2 text-success f6 d-flex flex-items-center gap-2"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          <CheckCircleFillIcon size={15} />
          <span>{patchStatus}</span>
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

      {!report ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <ShieldCheckIcon size={24} className="text-accent" />
          </div>
          <div className="f4 text-bold">{t("security.emptyGateReady")}</div>
          <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 480, margin: "8px auto" }}>
            {t("security.emptyGateDesc")}
          </p>
          <button
            type="button"
            className="btn btn-primary d-inline-flex flex-items-center gap-1"
            disabled={scanning || !repoPath.trim()}
            onClick={runScan}
          >
            <PlayIcon size={14} />
            {scanning ? t("common.loading") : t("security.runScan")}
          </button>
        </div>
      ) : (
        <div className="d-flex flex-column gap-2">
          <div
            className="primer-card p-3 d-flex flex-justify-between flex-items-center flex-wrap gap-2"
            style={{
              borderColor: report.passed
                ? "var(--color-success-fg)"
                : "var(--color-danger-fg)",
            }}
          >
            <div className="d-flex flex-items-center gap-2">
              {report.passed ? (
                <CheckCircleFillIcon size={18} className="text-success" />
              ) : (
                <AlertIcon size={18} className="text-danger" />
              )}
              <div>
                <div className="f5 text-bold">
                  {report.passed
                    ? t("security.gatePassed")
                    : t("security.gateBlocked")}
                </div>
                <div className="f6 text-muted tabular-nums">
                  {t("security.scannedIn")} {report.scan_duration_ms} ms • Critical:{" "}
                  {report.critical_count} • High: {report.high_count} • Medium:{" "}
                  {report.medium_count} • Low: {report.low_count}
                </div>
              </div>
            </div>
            <span className="badge tabular-nums">
              {report.total_findings} {t("security.findings")}
            </span>
          </div>

          {report.findings.map((finding) => (
            <div key={finding.id} className="primer-card p-3">
              <div className="d-flex flex-justify-between flex-items-start gap-2 flex-wrap">
                <div>
                  <div className="d-flex flex-items-center gap-2 flex-wrap">
                    <span
                      className={`badge text-bold ${severityClass(
                        finding.severity
                      )}`}
                    >
                      {finding.severity.toUpperCase()}
                    </span>
                    <strong className="f5">{finding.title}</strong>
                    {finding.cwe_id && (
                      <span className="badge text-mono">{finding.cwe_id}</span>
                    )}
                    {finding.owasp_category && (
                      <span className="badge">{finding.owasp_category}</span>
                    )}
                  </div>
                  <div className="f6 text-mono text-muted mt-1 tabular-nums">
                    {finding.file_path} • Line {finding.line_start}
                  </div>
                </div>

                {finding.suggested_fix && (
                  <button
                    type="button"
                    className="btn btn-primary f6 d-inline-flex flex-items-center gap-1"
                    onClick={() => applyPatch(finding)}
                  >
                    <SparklesFillIcon size={13} />
                    {t("security.applyAiFix")}
                  </button>
                )}
              </div>

              <p className="f6 text-muted mt-2 mb-2">{finding.description}</p>
              <pre className="merge-pane-code p-2 m-0 overflow-auto">
                {finding.vulnerable_snippet}
              </pre>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SecurityScannerView;
