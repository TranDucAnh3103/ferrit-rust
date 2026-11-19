import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UserPluginRecord } from "../types/ipc";
import {
  PlugIcon,
  TrashIcon,
  SyncIcon,
  ShieldIcon,
  PackageIcon,
} from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";

interface PluginsViewProps {
  onNavigateToMarketplace?: () => void;
}

export const PluginsView: React.FC<PluginsViewProps> = ({
  onNavigateToMarketplace,
}) => {
  const { t } = useI18n();
  const [plugins, setPlugins] = useState<UserPluginRecord[]>([]);
  const [loading, setLoading] = useState<boolean>(true);

  const loadPlugins = async () => {
    setLoading(true);
    try {
      const res = await invoke<UserPluginRecord[]>("plugin_list_installed");
      setPlugins(res);
    } catch (err) {
      console.error("Failed to load plugins:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPlugins();
  }, []);

  const handleToggle = async (plugin: UserPluginRecord) => {
    const nextState = !plugin.is_enabled;
    try {
      await invoke("plugin_toggle_active", {
        pluginId: plugin.id,
        enabled: nextState,
      });
      setPlugins((prev) =>
        prev.map((p) => (p.id === plugin.id ? { ...p, is_enabled: nextState } : p))
      );
    } catch (err) {
      alert(`Failed to toggle plugin: ${err}`);
    }
  };

  const handleUninstall = async (pluginId: string) => {
    if (!confirm(t("plugins.uninstallConfirm"))) {
      return;
    }
    try {
      await invoke("plugin_uninstall", { pluginId });
      setPlugins((prev) => prev.filter((p) => p.id !== pluginId));
    } catch (err) {
      alert(`Failed to uninstall: ${err}`);
    }
  };

  return (
    <div
      style={{
        padding: 24,
        display: "flex",
        flexDirection: "column",
        gap: 20,
        height: "100%",
        overflowY: "auto",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <h2 style={{ margin: 0, fontSize: 20, fontWeight: 600, display: "flex", alignItems: "center", gap: 10 }}>
            <PlugIcon size={22} />
            {t("plugins.title")}
          </h2>
          <p style={{ margin: "4px 0 0", fontSize: 13, color: "var(--color-fg-muted)" }}>
            {t("plugins.subtitle")}
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          {onNavigateToMarketplace && (
            <button
              onClick={onNavigateToMarketplace}
              style={{
                padding: "6px 14px",
                borderRadius: 6,
                border: "1px solid var(--color-btn-primary-border)",
                backgroundColor: "var(--color-btn-primary-bg)",
                color: "var(--color-btn-primary-text)",
                cursor: "pointer",
                fontSize: 13,
                fontWeight: 600,
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <PackageIcon size={14} />
              {t("plugins.browseMarketplace")}
            </button>
          )}

          <button
            onClick={loadPlugins}
            disabled={loading}
            style={{
              padding: "6px 12px",
              borderRadius: 6,
              border: "1px solid var(--color-btn-border)",
              backgroundColor: "var(--color-btn-bg)",
              color: "var(--color-btn-text)",
              cursor: loading ? "not-allowed" : "pointer",
              fontSize: 13,
            }}
          >
            <SyncIcon size={14} className={loading ? "spin" : ""} />
          </button>
        </div>
      </div>

      {plugins.length === 0 && !loading ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <PlugIcon size={24} className="text-accent" />
          </div>
          <div className="f4 text-bold">{t("plugins.emptyPlugins")}</div>
          <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 440, margin: "8px auto" }}>
            {t("plugins.emptyPluginsDesc")}
          </p>
          {onNavigateToMarketplace && (
            <button
              onClick={onNavigateToMarketplace}
              className="btn btn-primary d-inline-flex flex-items-center gap-1"
            >
              <PackageIcon size={14} /> {t("plugins.browseMarketplace")}
            </button>
          )}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {plugins.map((plugin) => (
            <div
              key={plugin.id}
              style={{
                backgroundColor: "var(--color-canvas-default)",
                border: "1px solid var(--color-border-default)",
                borderRadius: 8,
                padding: 16,
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 16,
              }}
            >
              <div style={{ display: "flex", alignItems: "flex-start", gap: 14 }}>
                <div
                  style={{
                    width: 40,
                    height: 40,
                    borderRadius: 8,
                    backgroundColor: plugin.is_enabled
                      ? "var(--color-success-subtle)"
                      : "var(--color-canvas-subtle)",
                    color: plugin.is_enabled
                      ? "var(--color-success-fg)"
                      : "var(--color-fg-muted)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    marginTop: 2,
                  }}
                >
                  <PlugIcon size={20} />
                </div>

                <div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <h4 style={{ margin: 0, fontSize: 15, fontWeight: 600 }}>{plugin.name}</h4>
                    <span
                      style={{
                        fontSize: 11,
                        padding: "1px 6px",
                        borderRadius: 10,
                        backgroundColor: plugin.is_enabled
                          ? "var(--color-success-subtle)"
                          : "var(--color-canvas-subtle)",
                        color: plugin.is_enabled
                          ? "var(--color-success-fg)"
                          : "var(--color-fg-muted)",
                        fontWeight: 500,
                      }}
                    >
                      {plugin.is_enabled ? "Active" : "Disabled"}
                    </span>
                  </div>

                  <div style={{ fontSize: 12, color: "var(--color-fg-muted)", margin: "4px 0 8px" }}>
                    ID: <code>{plugin.id}</code> · v{plugin.version} · Author: {plugin.author}
                  </div>

                  {/* Capability Badges */}
                  <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
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
                      <ShieldIcon size={12} />
                      Heap Max: {plugin.capabilities.max_memory_mb} MB
                    </span>

                    {plugin.capabilities.git_read && (
                      <span
                        style={{
                          fontSize: 11,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: "var(--color-accent-subtle)",
                          color: "var(--color-accent-fg)",
                        }}
                      >
                        Git Read
                      </span>
                    )}

                    {plugin.capabilities.commit_hook && (
                      <span
                        style={{
                          fontSize: 11,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: "var(--color-attention-subtle)",
                          color: "var(--color-attention-fg)",
                        }}
                      >
                        Commit Hook
                      </span>
                    )}

                    {plugin.capabilities.ai_prompt_filter && (
                      <span
                        style={{
                          fontSize: 11,
                          padding: "2px 6px",
                          borderRadius: 4,
                          backgroundColor: "var(--color-done-subtle)",
                          color: "var(--color-done-fg)",
                        }}
                      >
                        AI Filter
                      </span>
                    )}
                  </div>
                </div>
              </div>

              {/* Action Controls */}
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <button
                  onClick={() => handleToggle(plugin)}
                  style={{
                    padding: "6px 14px",
                    borderRadius: 6,
                    fontSize: 12,
                    fontWeight: 500,
                    cursor: "pointer",
                    border: "1px solid var(--color-border-default)",
                    backgroundColor: plugin.is_enabled
                      ? "var(--color-canvas-subtle)"
                      : "var(--color-btn-primary-bg)",
                    color: plugin.is_enabled
                      ? "var(--color-fg-default)"
                      : "var(--color-btn-primary-text)",
                  }}
                >
                  {plugin.is_enabled ? "Disable" : "Enable"}
                </button>

                <button
                  onClick={() => handleUninstall(plugin.id)}
                  title="Uninstall plugin package"
                  style={{
                    padding: "6px 10px",
                    borderRadius: 6,
                    border: "1px solid var(--color-border-default)",
                    backgroundColor: "transparent",
                    color: "var(--color-danger-fg)",
                    cursor: "pointer",
                  }}
                >
                  <TrashIcon size={14} />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
