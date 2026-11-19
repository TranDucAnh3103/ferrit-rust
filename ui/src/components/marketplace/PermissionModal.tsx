import React from "react";
import { MarketplacePluginEntry } from "../../types/ipc";
import { ShieldCheckIcon, AlertIcon, CheckIcon, XIcon } from "@primer/octicons-react";

interface PermissionModalProps {
  plugin: MarketplacePluginEntry;
  onConfirm: () => void;
  onCancel: () => void;
  isInstalling: boolean;
}

export const PermissionModal: React.FC<PermissionModalProps> = ({
  plugin,
  onConfirm,
  onCancel,
  isInstalling,
}) => {
  const hasGitRead = plugin.capabilities.includes("git_read");
  const hasGitWrite = plugin.capabilities.includes("git_write");
  const hasCommitHook = plugin.capabilities.includes("commit_hook");
  const hasAiPromptFilter = plugin.capabilities.includes("ai_prompt_filter");
  const hasNetwork = plugin.capabilities.includes("network_access");

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: "rgba(0, 0, 0, 0.65)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
        backdropFilter: "blur(2px)",
      }}
    >
      <div
        style={{
          width: 520,
          backgroundColor: "var(--color-canvas-default)",
          border: "1px solid var(--color-border-default)",
          borderRadius: 8,
          boxShadow: "var(--color-shadow-large)",
          padding: 24,
          display: "flex",
          flexDirection: "column",
          gap: 16,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <div
            style={{
              width: 36,
              height: 36,
              borderRadius: 6,
              backgroundColor: "var(--color-attention-subtle)",
              color: "var(--color-attention-fg)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <ShieldCheckIcon size={20} />
          </div>
          <div>
            <h3 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>
              Install Plugin: {plugin.name}
            </h3>
            <span style={{ fontSize: 12, color: "var(--color-fg-muted)" }}>
              by {plugin.author} (v{plugin.version})
            </span>
          </div>
        </div>

        <p style={{ margin: 0, fontSize: 13, color: "var(--color-fg-default)" }}>
          This WebAssembly extension requests the following sandbox capabilities. Please review and authorize:
        </p>

        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 10,
            padding: 12,
            backgroundColor: "var(--color-canvas-subtle)",
            borderRadius: 6,
            border: "1px solid var(--color-border-subtle)",
          }}
        >
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ color: hasGitRead ? "var(--color-success-fg)" : "var(--color-fg-muted)" }}>
              {hasGitRead ? <CheckIcon size={16} /> : <XIcon size={16} />}
            </span>
            <div style={{ fontSize: 12 }}>
              <strong>Read Git Repository State:</strong>
              <div style={{ color: "var(--color-fg-muted)" }}>
                {hasGitRead
                  ? "Access current branch name and list of staged files."
                  : "Denied - plugin cannot read repository status."}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ color: hasCommitHook ? "var(--color-success-fg)" : "var(--color-fg-muted)" }}>
              {hasCommitHook ? <CheckIcon size={16} /> : <XIcon size={16} />}
            </span>
            <div style={{ fontSize: 12 }}>
              <strong>Intercept Commit Lifecycle (Hooks):</strong>
              <div style={{ color: "var(--color-fg-muted)" }}>
                {hasCommitHook
                  ? "Can inspect, validate, or reject commit messages."
                  : "Denied - plugin does not hook into commit events."}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ color: hasAiPromptFilter ? "var(--color-success-fg)" : "var(--color-fg-muted)" }}>
              {hasAiPromptFilter ? <CheckIcon size={16} /> : <XIcon size={16} />}
            </span>
            <div style={{ fontSize: 12 }}>
              <strong>AI Prompt Filter:</strong>
              <div style={{ color: "var(--color-fg-muted)" }}>
                {hasAiPromptFilter
                  ? "Can sanitize and filter prompts sent to Cloud AI."
                  : "Denied - plugin does not intercept AI prompts."}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ color: hasGitWrite ? "var(--color-danger-fg)" : "var(--color-fg-muted)" }}>
              {hasGitWrite ? <AlertIcon size={16} /> : <XIcon size={16} />}
            </span>
            <div style={{ fontSize: 12 }}>
              <strong>Mutate Git State:</strong>
              <div style={{ color: "var(--color-fg-muted)" }}>
                {hasGitWrite ? "Can commit or modify local git references." : "Denied - read-only sandbox."}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", alignItems: "flex-start", gap: 10 }}>
            <span style={{ color: hasNetwork ? "var(--color-danger-fg)" : "var(--color-fg-muted)" }}>
              {hasNetwork ? <AlertIcon size={16} /> : <XIcon size={16} />}
            </span>
            <div style={{ fontSize: 12 }}>
              <strong>External Network Access:</strong>
              <div style={{ color: "var(--color-fg-muted)" }}>
                {hasNetwork ? "Can open outbound HTTP connections." : "Denied - no outbound network sockets."}
              </div>
            </div>
          </div>
        </div>

        <div
          style={{
            fontSize: 11,
            color: "var(--color-fg-muted)",
            display: "flex",
            justifyContent: "space-between",
          }}
        >
          <span>Linear Memory: Max 32 MB</span>
          <span>Publisher: {plugin.verified ? "Verified Ed25519" : "Community"}</span>
        </div>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 8 }}>
          <button
            onClick={onCancel}
            disabled={isInstalling}
            style={{
              padding: "6px 14px",
              borderRadius: 6,
              border: "1px solid var(--color-btn-border)",
              backgroundColor: "var(--color-btn-bg)",
              color: "var(--color-btn-text)",
              cursor: "pointer",
              fontSize: 13,
            }}
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            disabled={isInstalling}
            style={{
              padding: "6px 16px",
              borderRadius: 6,
              border: "1px solid var(--color-btn-primary-border)",
              backgroundColor: "var(--color-btn-primary-bg)",
              color: "var(--color-btn-primary-text)",
              cursor: isInstalling ? "not-allowed" : "pointer",
              fontWeight: 600,
              fontSize: 13,
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            {isInstalling ? "Verifying & Installing..." : "Authorize & Install"}
          </button>
        </div>
      </div>
    </div>
  );
};
