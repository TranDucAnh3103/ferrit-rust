import React from "react";
import {
  CheckCircleFillIcon,
  XCircleFillIcon,
  ClockIcon,
  SkipIcon,
  TrashIcon,
  GlobeIcon,
  ShieldCheckIcon,
} from "@primer/octicons-react";
import type {
  RemoteEndpoint,
  RemoteHealthReport,
  RemotePushResult,
} from "../types/ipc";

interface RemoteSyncCardProps {
  endpoint: RemoteEndpoint;
  progressPercent?: number;
  lastResult?: RemotePushResult;
  health?: RemoteHealthReport;
  onToggleEnabled: (endpoint: RemoteEndpoint) => void;
  onDelete: (remoteName: string) => void;
}

export const RemoteSyncCard: React.FC<RemoteSyncCardProps> = ({
  endpoint,
  progressPercent,
  lastResult,
  health,
  onToggleEnabled,
  onDelete,
}) => {
  const renderStatusIcon = () => {
    if (lastResult) {
      switch (lastResult.status) {
        case "success":
          return <CheckCircleFillIcon size={15} className="text-success" />;
        case "timed_out":
          return <ClockIcon size={15} className="text-attention" />;
        case "skipped":
          return <SkipIcon size={15} className="text-muted" />;
        default:
          return <XCircleFillIcon size={15} className="text-danger" />;
      }
    }
    if (health) {
      if (health.status === "healthy") {
        return <CheckCircleFillIcon size={15} className="text-success" />;
      }
      if (health.status === "timeout") {
        return <ClockIcon size={15} className="text-attention" />;
      }
      return <XCircleFillIcon size={15} className="text-danger" />;
    }
    return <GlobeIcon size={15} className="text-muted" />;
  };

  return (
    <div className="primer-card p-3 interactive-row">
      <div className="d-flex flex-justify-between flex-items-start gap-2">
        <div className="d-flex flex-items-center gap-2 min-width-0">
          {renderStatusIcon()}
          <strong className="f5 text-primary">{endpoint.name}</strong>
          <span className="badge text-capitalize">{endpoint.provider}</span>
          <span className="badge text-capitalize">
            {endpoint.sync_policy.replace("_", " ")}
          </span>
          {endpoint.auth_account && (
            <span className="badge d-inline-flex flex-items-center gap-1">
              <ShieldCheckIcon size={12} />
              {endpoint.auth_account}
            </span>
          )}
        </div>
        <div className="d-flex flex-items-center gap-2 flex-shrink-0">
          <label className="d-inline-flex flex-items-center gap-1 f6 text-muted cursor-pointer">
            <input
              type="checkbox"
              checked={endpoint.is_enabled}
              onChange={() => onToggleEnabled(endpoint)}
            />
            Enabled
          </label>
          <button
            type="button"
            className="btn-octicon text-danger"
            aria-label={`Delete remote ${endpoint.name}`}
            onClick={() => onDelete(endpoint.name)}
          >
            <TrashIcon size={14} />
          </button>
        </div>
      </div>

      <div
        className="text-mono f6 text-muted mt-1 text-truncate"
        title={endpoint.url}
      >
        {endpoint.url}
      </div>

      {typeof progressPercent === "number" && (
        <div className="mt-2">
          <div
            className="rounded-1 overflow-hidden"
            style={{ height: 5, backgroundColor: "var(--color-canvas-inset)" }}
          >
            <div
              style={{
                width: `${Math.min(100, Math.max(0, progressPercent))}%`,
                height: "100%",
                backgroundColor:
                  lastResult && !lastResult.is_success
                    ? "var(--color-danger-fg)"
                    : "var(--color-success-fg)",
                transition: "width 180ms ease",
              }}
            />
          </div>
        </div>
      )}

      {(lastResult || health) && (
        <div className="d-flex flex-justify-between flex-items-center mt-2 f6">
          <span
            className={
              lastResult?.error_message
                ? "text-danger"
                : health && health.status !== "healthy"
                ? "text-danger"
                : "text-muted"
            }
          >
            {lastResult
              ? lastResult.error_message ??
                `Synced (${lastResult.status.replace("_", " ")})`
              : health?.message}
          </span>
          <span className="tabular-nums text-muted text-mono">
            {lastResult
              ? `${lastResult.duration_ms} ms`
              : health
              ? `${health.latency_ms} ms`
              : ""}
          </span>
        </div>
      )}
    </div>
  );
};
