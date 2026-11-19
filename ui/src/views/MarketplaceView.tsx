import React, { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  MarketplaceIndex,
  MarketplacePluginEntry,
  UserPluginRecord,
} from "../types/ipc";
import { PluginCard } from "../components/marketplace/PluginCard";
import { PermissionModal } from "../components/marketplace/PermissionModal";
import {
  SearchIcon,
  SyncIcon,
  PackageIcon,
  AlertIcon,
} from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";

export const MarketplaceView: React.FC = () => {
  const { t } = useI18n();
  const [catalog, setCatalog] = useState<MarketplaceIndex | null>(null);
  const [installed, setInstalled] = useState<UserPluginRecord[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [selectedCategory, setSelectedCategory] = useState<string>("all");
  const [selectedPlugin, setSelectedPlugin] = useState<MarketplacePluginEntry | null>(null);
  const [isInstalling, setIsInstalling] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const loadData = async () => {
    setLoading(true);
    setErrorMessage(null);
    try {
      const cat = await invoke<MarketplaceIndex>("marketplace_fetch_catalog", {
        customUrl: null,
      });
      setCatalog(cat);

      const inst = await invoke<UserPluginRecord[]>("plugin_list_installed");
      setInstalled(inst);
    } catch (err: any) {
      setErrorMessage(typeof err === "string" ? err : err.message || "Failed to load marketplace catalog");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleInstallConfirm = async () => {
    if (!selectedPlugin) return;
    setIsInstalling(true);
    try {
      await invoke("marketplace_install_plugin", { entry: selectedPlugin });
      setSelectedPlugin(null);
      await loadData();
    } catch (err: any) {
      alert(`Installation failed: ${typeof err === "string" ? err : err.message || "Verification error"}`);
    } finally {
      setIsInstalling(false);
    }
  };

  const installedMap = useMemo(() => {
    const map = new Set<string>();
    for (const p of installed) {
      map.add(p.id);
    }
    return map;
  }, [installed]);

  const filteredPlugins = useMemo(() => {
    if (!catalog) return [];
    return catalog.plugins.filter((p) => {
      const matchesCategory =
        selectedCategory === "all" || p.category === selectedCategory;
      const matchesSearch =
        p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        p.author.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesCategory && matchesSearch;
    });
  }, [catalog, selectedCategory, searchQuery]);

  const categories: { id: string; label: string }[] = [
    { id: "all", label: "All Categories" },
    { id: "linter", label: "Linters & Hooks" },
    { id: "ai_prompt", label: "AI Prompts" },
    { id: "automation", label: "Automations" },
    { id: "webhook", label: "Webhooks" },
    { id: "ui_theme", label: "Themes" },
  ];

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
            <PackageIcon size={22} />
            {t("marketplace.title")}
          </h2>
          <p style={{ margin: "4px 0 0", fontSize: 13, color: "var(--color-fg-muted)" }}>
            {t("marketplace.subtitle")}
          </p>
        </div>

        <button
          onClick={loadData}
          disabled={loading}
          style={{
            padding: "6px 14px",
            borderRadius: 6,
            border: "1px solid var(--color-btn-border)",
            backgroundColor: "var(--color-btn-bg)",
            color: "var(--color-btn-text)",
            cursor: loading ? "not-allowed" : "pointer",
            fontSize: 13,
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          <SyncIcon size={14} className={loading ? "spin" : ""} />
          {t("common.refresh")}
        </button>
      </div>

      {errorMessage && (
        <div
          style={{
            padding: "12px 16px",
            backgroundColor: "var(--color-danger-subtle)",
            border: "1px solid var(--color-danger-muted)",
            borderRadius: 6,
            color: "var(--color-danger-fg)",
            fontSize: 13,
            display: "flex",
            alignItems: "center",
            gap: 10,
          }}
        >
          <AlertIcon size={16} />
          <span>{errorMessage}</span>
        </div>
      )}

      {/* Search & Filter Toolbar */}
      <div style={{ display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <div
          style={{
            position: "relative",
            flex: 1,
            minWidth: 240,
          }}
        >
          <span
            style={{
              position: "absolute",
              left: 10,
              top: 8,
              color: "var(--color-fg-muted)",
            }}
          >
            <SearchIcon size={16} />
          </span>
          <input
            type="text"
            placeholder={t("marketplace.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              width: "100%",
              padding: "7px 10px 7px 32px",
              borderRadius: 6,
              border: "1px solid var(--color-border-default)",
              backgroundColor: "var(--color-canvas-default)",
              color: "var(--color-fg-default)",
              fontSize: 13,
              outline: "none",
            }}
          />
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
          {categories.map((c) => {
            const active = selectedCategory === c.id;
            return (
              <button
                key={c.id}
                onClick={() => setSelectedCategory(c.id)}
                style={{
                  padding: "5px 12px",
                  borderRadius: 16,
                  border: active
                    ? "1px solid var(--color-accent-emphasis)"
                    : "1px solid var(--color-border-default)",
                  backgroundColor: active
                    ? "var(--color-accent-subtle)"
                    : "var(--color-canvas-default)",
                  color: active
                    ? "var(--color-accent-fg)"
                    : "var(--color-fg-default)",
                  cursor: "pointer",
                  fontSize: 12,
                  fontWeight: active ? 600 : 400,
                }}
              >
                {c.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Grid of Plugins */}
      {loading ? (
        <div
          style={{
            textAlign: "center",
            padding: 48,
            color: "var(--color-fg-muted)",
            fontSize: 13,
          }}
        >
          Loading marketplace catalog...
        </div>
      ) : filteredPlugins.length === 0 ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <PackageIcon size={24} className="text-muted" />
          </div>
          <div className="f4 text-bold">No Extensions Found</div>
          <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 440, margin: "8px auto" }}>
            No community extensions matched your search or category filter. Try searching with different terms or selecting "All Categories".
          </p>
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            onClick={() => {
              setSearchQuery("");
              setSelectedCategory("all");
            }}
          >
            Clear Filters
          </button>
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(320px, 1fr))",
            gap: 16,
          }}
        >
          {filteredPlugins.map((plugin) => (
            <PluginCard
              key={plugin.id}
              plugin={plugin}
              isInstalled={installedMap.has(plugin.id)}
              onInstallClick={(p) => setSelectedPlugin(p)}
            />
          ))}
        </div>
      )}

      {selectedPlugin && (
        <PermissionModal
          plugin={selectedPlugin}
          onConfirm={handleInstallConfirm}
          onCancel={() => setSelectedPlugin(null)}
          isInstalling={isInstalling}
        />
      )}
    </div>
  );
};
