import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  ShieldLockIcon,
  TerminalIcon,
  CheckCircleFillIcon,
  SyncIcon,
  CheckIcon,
} from "@primer/octicons-react";
import { Palette, Save, Globe } from "lucide-react";
import { AppSettings, GhCliStatus, THEME_OPTIONS, ThemeMode, formatIpcError } from "../types/ipc";
import { WebhooksSettings } from "../components/settings/WebhooksSettings";
import { useI18n } from "../i18n/I18nContext";

interface SettingsViewProps {
  settings: AppSettings;
  onSettingsUpdated: (s: AppSettings) => void;
  onVaultChange: () => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  settings,
  onSettingsUpdated,
  onVaultChange,
}) => {
  const { lang, setLang, t } = useI18n();
  const [form, setForm] = useState<AppSettings>(settings);
  const [patInput, setPatInput] = useState("");
  const [aiKeyInput, setAiKeyInput] = useState("");
  const [ghStatus, setGhStatus] = useState<GhCliStatus | null>(null);
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    setForm(settings);
  }, [settings]);

  useEffect(() => {
    invoke<GhCliStatus>("ghcli_check_status")
      .then(setGhStatus)
      .catch(() => {});
  }, []);

  const saveAll = async () => {
    setErrorMsg(null);
    setStatusMsg(null);
    try {
      const updatedSettings = { ...form, language: lang === "vi" ? "vi-VN" : "en-US" };
      await invoke("system_save_settings", { settings: updatedSettings });
      if (patInput.trim()) {
        await invoke("auth_save_token", { accountId: "default", token: patInput.trim() });
        setPatInput("");
      }
      if (aiKeyInput.trim()) {
        await invoke("auth_save_ai_key", {
          provider: form.cloud_ai_provider,
          apiKey: aiKeyInput.trim(),
        });
        setAiKeyInput("");
      }
      onSettingsUpdated(updatedSettings);
      onVaultChange();
      setStatusMsg(t("settings.savedSuccess"));
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const syncFromGh = async () => {
    setErrorMsg(null);
    try {
      const ok = await invoke<boolean>("auth_sync_from_gh", { accountId: "default" });
      onVaultChange();
      setStatusMsg(ok ? "Imported active token from `gh auth token` into Windows Keyring!" : "No token found in `gh`.");
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const installGh = async () => {
    setErrorMsg(null);
    setStatusMsg("Downloading official portable gh CLI & verifying SHA256...");
    try {
      const status = await invoke<GhCliStatus>("ghcli_install_silent");
      setGhStatus(status);
      setStatusMsg(`GitHub CLI ${status.version || ""} installed in sandbox!`);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3 pb-5" style={{ maxWidth: 920 }}>
      {/* Header Banner */}
      <div className="d-flex flex-items-center flex-justify-between pb-2 border-bottom border-muted">
        <div>
          <h2 className="f3 text-bold m-0 text-primary">{t("settings.title")}</h2>
          <p className="text-small text-muted m-0 mt-1">{t("settings.subtitle")}</p>
        </div>
        <button className="btn btn-primary" onClick={saveAll}>
          <Save size={15} />
          <span>{t("settings.saveAll")}</span>
        </button>
      </div>

      {errorMsg && <div className="flash flash-error p-3 f6 rounded-2">{errorMsg}</div>}
      {statusMsg && <div className="flash flash-success p-3 f6 rounded-2">{statusMsg}</div>}

      {/* 1. Language & Localization Card */}
      <div className="primer-surface p-4">
        <div className="d-flex flex-items-center gap-2 mb-2">
          <Globe size={18} className="text-accent" />
          <h3 className="f4 text-bold m-0">{t("settings.langSectionTitle")}</h3>
        </div>
        <p className="text-small text-muted mb-3">{t("settings.langSectionDesc")}</p>

        <div className="d-flex gap-3">
          {/* Vietnamese option */}
          <button
            type="button"
            onClick={() => setLang("vi")}
            className={`flex-1 p-3 text-left border rounded-2 transition-colors ${
              lang === "vi"
                ? "border-accent bg-accent-subtle"
                : "border-muted bg-canvas-inset hover-bg-subtle"
            }`}
            style={{ cursor: "pointer" }}
          >
            <div className="d-flex flex-items-center flex-justify-between mb-1">
              <span className="text-bold f5 text-primary">{t("settings.langVi")}</span>
              {lang === "vi" && <CheckIcon size={16} className="text-accent" />}
            </div>
            <div className="text-small text-muted">{t("settings.langViDesc")}</div>
          </button>

          {/* English option */}
          <button
            type="button"
            onClick={() => setLang("en")}
            className={`flex-1 p-3 text-left border rounded-2 transition-colors ${
              lang === "en"
                ? "border-accent bg-accent-subtle"
                : "border-muted bg-canvas-inset hover-bg-subtle"
            }`}
            style={{ cursor: "pointer" }}
          >
            <div className="d-flex flex-items-center flex-justify-between mb-1">
              <span className="text-bold f5 text-primary">{t("settings.langEn")}</span>
              {lang === "en" && <CheckIcon size={16} className="text-accent" />}
            </div>
            <div className="text-small text-muted">{t("settings.langEnDesc")}</div>
          </button>
        </div>
      </div>

      {/* 2. Credential Vault + GitHub CLI Sandbox */}
      <div className="primer-surface p-4">
        <div className="d-flex flex-items-center gap-2 mb-2">
          <ShieldLockIcon size={18} className="text-success" />
          <h3 className="f4 text-bold m-0">{t("settings.vaultTitle")}</h3>
        </div>
        <p className="text-small text-muted mb-3">{t("settings.vaultDesc")}</p>

        <div className="d-flex gap-3 mb-3">
          <input
            type="password"
            className="primer-input flex-1"
            placeholder={t("settings.patPlaceholder")}
            value={patInput}
            onChange={(e) => setPatInput(e.target.value)}
          />
          <button className="btn" onClick={syncFromGh} style={{ minWidth: 170 }}>
            <SyncIcon size={14} />
            <span>{t("settings.syncGh")}</span>
          </button>
        </div>

        <div className="d-flex flex-items-center flex-justify-between p-3 bg-canvas-inset rounded-2 border border-muted f6">
          <div className="d-flex flex-items-center gap-2">
            <TerminalIcon size={16} className="text-primary flex-shrink-0" />
            {ghStatus?.is_installed ? (
              <span className="d-flex flex-items-center gap-1">
                <CheckCircleFillIcon size={14} className="text-success" />
                <span>
                  `gh` v{ghStatus.version || "ready"} ({ghStatus.is_sandboxed ? "App Sandbox" : "System PATH"})
                </span>
              </span>
            ) : (
              <span className="text-muted">{t("settings.ghNotDetected")}</span>
            )}
          </div>
          {!ghStatus?.is_installed && (
            <button className="btn btn-sm btn-primary" onClick={installGh}>
              {t("settings.installGh")}
            </button>
          )}
        </div>
      </div>

      {/* 3. GitHub & Naming Config */}
      <div className="primer-surface p-4">
        <h3 className="f4 text-bold mb-3">{t("settings.accountTitle")}</h3>
        <div className="d-flex flex-wrap gap-3 mb-3">
          <div className="flex-1" style={{ minWidth: 260 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.username")}</label>
            <input
              className="primer-input width-full"
              value={form.github_username}
              onChange={(e) => setForm({ ...form, github_username: e.target.value })}
            />
          </div>
          <div className="flex-1" style={{ minWidth: 260 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.orgName")}</label>
            <input
              className="primer-input width-full"
              placeholder={t("settings.orgPlaceholder")}
              value={form.github_org_name}
              onChange={(e) => setForm({ ...form, github_org_name: e.target.value })}
            />
          </div>
        </div>

        <div className="d-flex flex-wrap gap-3 mb-3">
          <div className="flex-1" style={{ minWidth: 260 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.namingPattern")}</label>
            <input
              className="primer-input width-full text-mono"
              value={form.naming_pattern}
              onChange={(e) => setForm({ ...form, naming_pattern: e.target.value })}
            />
          </div>
          <div className="width-full mt-2">
            <label className="d-flex flex-items-center gap-1 f6 text-bold mb-2">
              <Palette size={14} className="text-accent" /> {t("settings.themeTitle")}
            </label>
            <div className="d-flex flex-wrap gap-3">
              {THEME_OPTIONS.map((opt) => {
                const selected = form.theme_mode === opt.id;
                return (
                  <button
                    key={opt.id}
                    type="button"
                    onClick={() => {
                      const nextForm = { ...form, theme_mode: opt.id as ThemeMode };
                      setForm(nextForm);
                      onSettingsUpdated(nextForm);
                    }}
                    className={`theme-option-btn border p-3 ${
                      selected ? "is-selected" : "border-muted bg-canvas-inset"
                    }`}
                    style={{ width: "calc(33.33% - 8px)", minWidth: 220 }}
                  >
                    <div>
                      <div className="f6 text-bold d-flex flex-items-center gap-1">
                        {opt.label}
                        {selected && <CheckIcon size={14} className="text-accent" />}
                      </div>
                      <div className="text-small text-muted mt-1">{opt.description}</div>
                    </div>
                    <div className="d-flex gap-1">
                      {opt.swatches.map((hex, i) => (
                        <span
                          key={i}
                          style={{
                            width: 10,
                            height: 10,
                            borderRadius: "50%",
                            backgroundColor: hex,
                            border: "1px solid var(--color-border-default)",
                          }}
                        />
                      ))}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        </div>

        <div className="d-flex flex-wrap gap-4 pt-2">
          <label className="d-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={form.repo_private}
              onChange={(e) => setForm({ ...form, repo_private: e.target.checked })}
            />
            <span>{t("settings.privateRepo")}</span>
          </label>
          <label className="d-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={form.safe_sync_enabled}
              onChange={(e) => setForm({ ...form, safe_sync_enabled: e.target.checked })}
            />
            <span>{t("settings.safeSync")}</span>
          </label>
          <label className="d-flex flex-items-center gap-2 f6 cursor-pointer">
            <input
              type="checkbox"
              checked={form.auto_stash_enabled}
              onChange={(e) => setForm({ ...form, auto_stash_enabled: e.target.checked })}
            />
            <span>{t("settings.autoStash")}</span>
          </label>
        </div>
      </div>

      {/* 4. Pure Cloud AI Config */}
      <div className="primer-surface p-4">
        <h3 className="f4 text-bold mb-3">{t("settings.aiTitle")}</h3>
        <div className="d-flex flex-wrap gap-3 mb-3">
          <div style={{ minWidth: 260 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.aiProvider")}</label>
            <select
              className="primer-input width-full"
              value={form.cloud_ai_provider}
              onChange={(e) => {
                const nextProvider = e.target.value;
                const nextModel = nextProvider === "xkiro" ? "mistralai/codestral-2508" : form.cloud_ai_model;
                setForm({ ...form, cloud_ai_provider: nextProvider, cloud_ai_model: nextModel });
              }}
            >
              <option value="xkiro">xKiro (Free Multi-Model Failover - Recommended)</option>
              <option value="gemini">Google Gemini (2.0 Flash / 1.5 Pro)</option>
              <option value="openai">OpenAI (GPT-4o / GPT-4o-mini)</option>
              <option value="claude">Anthropic Claude 3.5 Sonnet</option>
              <option value="groq">Groq LPU (Llama 3.3 70B)</option>
              <option value="openrouter">OpenRouter</option>
            </select>
          </div>
          <div className="flex-1" style={{ minWidth: 220 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.aiModel")}</label>
            <input
              className="primer-input width-full text-mono"
              value={form.cloud_ai_model}
              onChange={(e) => setForm({ ...form, cloud_ai_model: e.target.value })}
            />
          </div>
          <div className="flex-1" style={{ minWidth: 260 }}>
            <label className="d-block f6 text-bold mb-1">{t("settings.aiKeyPlaceholder")}</label>
            <input
              type="password"
              className="primer-input width-full"
              placeholder={form.cloud_ai_provider === "xkiro" ? "Đã cấu hình key xKiro trong Windows Vault (nhập để ghi đè)" : "Paste API key..."}
              value={aiKeyInput}
              onChange={(e) => setAiKeyInput(e.target.value)}
            />
          </div>
        </div>

        {form.cloud_ai_provider === "xkiro" && (
          <div className="p-3 mb-3 bg-canvas-inset border border-muted rounded-2 text-small">
            <div className="d-flex flex-items-center gap-2 text-bold text-success mb-1">
              <span>●</span>
              <span>Chuỗi Tự Động Failover Đa Model (Hình 3, 4, 5 Free Tier):</span>
            </div>
            <div className="text-muted text-mono f6" style={{ wordBreak: "break-all" }}>
              mistralai/codestral-2508 → mistralai/devstral-medium → cohere/north-mini-code → mistralai/mistral-small-2603 → cohere/command-a → cohere/command-r7b-12-2024 → sensenova/sensenova-6.8-flash-lite → cohere/aya-expanse-32b → mistralai/ministral-14b
            </div>
            <div className="text-muted mt-1 f6">
              Hệ thống tự động thử lần lượt các model trong chuỗi nếu gặp giới hạn tốc độ (429), lỗi mạng, hoặc server quá tải.
            </div>
          </div>
        )}
      </div>

      {/* 5. Multi-Channel Webhooks & Dispatcher */}
      <div className="primer-surface p-4">
        <WebhooksSettings />
      </div>

      {/* 6. Sticky Save Action Bar */}
      <div className="d-flex flex-items-center flex-justify-end gap-3 pt-2">
        <button className="btn btn-lg btn-primary" onClick={saveAll}>
          <Save size={16} />
          <span>{t("settings.saveAll")}</span>
        </button>
      </div>
    </div>
  );
};

export default SettingsView;
