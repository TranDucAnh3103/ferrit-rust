import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  PulseIcon,
  BookIcon,
  CheckCircleFillIcon,
  XCircleFillIcon,
  AlertIcon,
  SyncIcon,
  ShieldCheckIcon,
  RepoIcon,
} from "@primer/octicons-react";
import { TokenReport, formatIpcError } from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

export const TokenReportView: React.FC = () => {
  const { t } = useI18n();
  const [report, setReport] = useState<TokenReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const load = async () => {
    setErrorMsg(null);
    setLoading(true);
    try {
      const r = await invoke<TokenReport>("repo_token_report", { accountId: "default" });
      setReport(r);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
  }, []);

  return (
    <div className="primer-surface p-4" style={{ maxWidth: 860 }}>
      <div className="d-flex flex-items-center flex-justify-between mb-3">
        <div className="d-flex flex-items-center gap-2">
          <PulseIcon size={18} className="text-accent" />
          <h2 className="f3 text-bold m-0">{t("report.title")}</h2>
        </div>
        <button className="btn btn-sm btn-secondary d-inline-flex flex-items-center gap-1" onClick={load} disabled={loading}>
          <SyncIcon size={14} /> {loading ? t("report.checking") : t("report.refresh")}
        </button>
      </div>

      {errorMsg && (
        <div className="flash flash-error p-2 f6 mb-3" role="alert">
          {errorMsg}
        </div>
      )}

      {report ? (
        <div className="d-flex flex-column gap-3 f5">
          <div className="d-flex flex-wrap gap-3">
            <div className="flex-1 p-3 bg-canvas-inset rounded-2 border border-muted">
              <div className="text-small text-muted mb-1">{t("report.identity")}</div>
              <div className="text-bold f4">{report.username}</div>
              <div className="text-small text-muted mt-1">
                Plan: {report.plan_name} &middot; 2FA:{" "}
                <span className={report.two_factor_authentication ? "text-success" : "text-attention"}>
                  {report.two_factor_authentication ? t("common.enabled") : t("common.disabled")}
                </span>
              </div>
            </div>

            <div className="flex-1 p-3 bg-canvas-inset rounded-2 border border-muted">
              <div className="text-small text-muted mb-1">{t("report.rateLimit")}</div>
              <div className="text-bold f4 tabular-nums">
                {report.rate_remaining} / {report.rate_limit}
              </div>
              <div className="text-small text-muted mt-1 text-mono">
                Resets: {report.rate_reset}
              </div>
            </div>

            <div className="flex-1 p-3 bg-canvas-inset rounded-2 border border-muted">
              <div className="text-small text-muted mb-1 d-flex flex-items-center gap-1">
                <RepoIcon size={12} /> {t("report.reposStorage")}
              </div>
              <div className="text-bold f4 tabular-nums">
                {report.public_repos} public / {report.private_repos} private
              </div>
              <div className="text-small text-muted mt-1 tabular-nums">{report.disk_usage} KB</div>
            </div>
          </div>

          <div className="p-3 bg-canvas-inset rounded-2 border border-muted">
            <div className="text-small text-bold mb-2 d-flex flex-items-center gap-1">
              <ShieldCheckIcon size={14} className="text-accent" /> {t("report.scopes")}
            </div>
            <div className="d-flex flex-wrap gap-2">
              <span className="Label d-inline-flex flex-items-center gap-1 px-2 py-1">
                {report.has_repo_scope ? (
                  <CheckCircleFillIcon size={12} className="text-success" />
                ) : (
                  <XCircleFillIcon size={12} className="text-danger" />
                )}
                repo
              </span>
              <span className="Label d-inline-flex flex-items-center gap-1 px-2 py-1">
                {report.has_delete_scope ? (
                  <CheckCircleFillIcon size={12} className="text-success" />
                ) : (
                  <AlertIcon size={12} className="text-attention" />
                )}
                delete_repo
              </span>
              <span className="Label d-inline-flex flex-items-center gap-1 px-2 py-1">
                {report.has_org_scope ? (
                  <CheckCircleFillIcon size={12} className="text-success" />
                ) : (
                  <AlertIcon size={12} className="text-muted" />
                )}
                read:org
              </span>
            </div>
          </div>
        </div>
      ) : (
        !errorMsg && (
          <div className="empty-state-card text-center p-4">
            <div className="empty-state-icon-box">
              <PulseIcon size={24} className="text-accent" />
            </div>
            <div className="f4 text-bold">{t("report.noDiagnostics")}</div>
            <p className="f6 text-muted mt-2 mb-0">
              {t("report.noDiagnosticsDesc")}
            </p>
          </div>
        )
      )}
    </div>
  );
};

export const UserGuideView: React.FC = () => {
  const { t } = useI18n();
  const [html, setHtml] = useState<string>("");

  useEffect(() => {
    invoke<string>("system_get_guide")
      .then(setHtml)
      .catch(() => setHtml("<p>Guide unavailable</p>"));
  }, []);

  return (
    <div className="primer-surface p-4 overflow-auto" style={{ maxHeight: "82vh", maxWidth: 940 }}>
      <div className="d-flex flex-items-center gap-2 mb-3 pb-2 border-bottom border-muted">
        <BookIcon size={18} className="text-accent" />
        <h2 className="f3 text-bold m-0">{t("guide.title")}</h2>
      </div>
      <div className="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
};
