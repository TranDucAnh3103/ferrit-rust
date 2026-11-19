import React from "react";
import { MarketplacePluginEntry } from "../../types/ipc";
import {
  VerifiedIcon,
  DownloadIcon,
  TagIcon,
  PackageIcon,
  CheckIcon,
} from "@primer/octicons-react";

interface PluginCardProps {
  plugin: MarketplacePluginEntry;
  isInstalled: boolean;
  onInstallClick: (plugin: MarketplacePluginEntry) => void;
}

export const PluginCard: React.FC<PluginCardProps> = ({
  plugin,
  isInstalled,
  onInstallClick,
}) => {
  return (
    <div
      style={{
        backgroundColor: "var(--color-canvas-default)",
        border: "1px solid var(--color-border-default)",
        borderRadius: 8,
        padding: 18,
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        gap: 12,
        transition: "border-color 0.15s ease, box-shadow 0.15s ease",
      }}
    >
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 6 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ color: "var(--color-accent-fg)" }}>
              <PackageIcon size={18} />
            </span>
            <h4 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>{plugin.name}</h4>
          </div>
          {plugin.verified && (
            <span
              style={{
                color: "var(--color-success-fg)",
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontSize: 11,
                fontWeight: 500,
                backgroundColor: "var(--color-success-subtle)",
                padding: "2px 8px",
                borderRadius: 12,
              }}
              title="Verified publisher cryptographic Ed25519 signature"
            >
              <VerifiedIcon size={12} />
              Verified
            </span>
          )}
        </div>

        <div style={{ fontSize: 12, color: "var(--color-fg-muted)", marginBottom: 8 }}>
          by {plugin.author} · v{plugin.version}
        </div>

        <p
          style={{
            margin: 0,
            fontSize: 13,
            color: "var(--color-fg-muted)",
            lineHeight: 1.45,
            minHeight: 38,
          }}
        >
          {plugin.description}
        </p>
      </div>

      <div>
        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap", marginBottom: 14 }}>
          <span
            style={{
              fontSize: 11,
              padding: "2px 6px",
              borderRadius: 4,
              backgroundColor: "var(--color-canvas-subtle)",
              border: "1px solid var(--color-border-subtle)",
              color: "var(--color-fg-muted)",
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
            }}
          >
            <TagIcon size={10} />
            {plugin.category}
          </span>
          {plugin.capabilities.map((cap) => (
            <span
              key={cap}
              style={{
                fontSize: 10,
                padding: "2px 6px",
                borderRadius: 4,
                backgroundColor: "var(--color-accent-subtle)",
                color: "var(--color-accent-fg)",
              }}
            >
              {cap}
            </span>
          ))}
        </div>

        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span style={{ fontSize: 11, color: "var(--color-fg-muted)" }}>
            {plugin.downloads_count.toLocaleString()} downloads
          </span>

          <button
            onClick={() => onInstallClick(plugin)}
            disabled={isInstalled}
            style={{
              padding: "5px 12px",
              borderRadius: 6,
              fontSize: 12,
              fontWeight: 500,
              cursor: isInstalled ? "default" : "pointer",
              border: isInstalled
                ? "1px solid var(--color-border-default)"
                : "1px solid var(--color-btn-primary-border)",
              backgroundColor: isInstalled
                ? "var(--color-canvas-subtle)"
                : "var(--color-btn-primary-bg)",
              color: isInstalled
                ? "var(--color-fg-muted)"
                : "var(--color-btn-primary-text)",
              display: "inline-flex",
              alignItems: "center",
              gap: 6,
            }}
          >
            {isInstalled ? (
              <>
                <CheckIcon size={14} />
                Installed
              </>
            ) : (
              <>
                <DownloadIcon size={14} />
                Install
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
