import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Header from "./components/Header";
import Sidebar from "./components/Sidebar";
import StephenCopilotDrawer from "./components/StephenCopilotDrawer";
import UploadSyncView from "./views/UploadSyncView";
import GitGraphView from "./views/GitGraphView";
import MultiRemoteView from "./views/MultiRemoteView";
import ConflictResolverView from "./views/ConflictResolverView";
import RepositoriesView from "./views/RepositoriesView";
import PullRequestsView from "./views/PullRequestsView";
import ActionsView from "./views/ActionsView";
import IssuesReleasesView from "./views/IssuesReleasesView";
import SshKeysView from "./views/SshKeysView";
import AnalyticsView from "./views/AnalyticsView";
import SecurityScannerView from "./views/SecurityScannerView";
import SemanticSearchView from "./views/SemanticSearchView";
import StaleBranchView from "./views/StaleBranchView";
import SyncScheduleSettingsView from "./views/SyncScheduleSettingsView";
import EnterpriseUpdatesView from "./views/EnterpriseUpdatesView";
import { PluginsView } from "./views/PluginsView";
import { MarketplaceView } from "./views/MarketplaceView";
import SettingsView from "./views/SettingsView";
import { TokenReportView, UserGuideView } from "./views/DiagnosticsGuideView";
import { THEME_OPTIONS, type ActiveTab, type AppSettings, type ThemeMode } from "./types/ipc";

const DEFAULT_SETTINGS: AppSettings = {
  github_username: "",
  github_org_name: "",
  naming_pattern: "session%02d-ex%02d-homework",
  theme_mode: "dark",
  git_commit_name: "",
  git_commit_email: "",
  repo_private: false,
  auto_push: false,
  default_commit_msg: "init",
  file_commit_msg_pattern: "{filename_no_ext}",
  sync_file_repo_name_with_commit: false,
  cloud_ai_provider: "gemini",
  cloud_ai_model: "gemini-2.0-flash",
  language: "vi-VN",
  safe_sync_enabled: true,
  auto_stash_enabled: true,
};

export default function App() {
  const [currentTab, setCurrentTab] = useState<ActiveTab>("upload");
  const [activeRepoPath, setActiveRepoPath] = useState<string>("");
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [theme, setTheme] = useState<ThemeMode>("dark");
  const [vaultActive, setVaultActive] = useState<boolean>(false);

  const refreshVaultStatus = () => {
    invoke<{ has_github_token: boolean }>("auth_list_accounts", { accountId: "default" })
      .then((res) => setVaultActive(res.has_github_token))
      .catch(() => setVaultActive(false));
  };

  const applyThemeAttributes = (mode: ThemeMode) => {
    const desc = THEME_OPTIONS.find((t) => t.id === mode);
    const polarity = desc ? desc.polarity : "dark";
    document.documentElement.setAttribute("data-color-mode", polarity);
    document.documentElement.setAttribute("data-dark-theme", mode);
    document.documentElement.setAttribute("data-light-theme", mode);
  };

  useEffect(() => {
    applyThemeAttributes(theme);
  }, [theme]);

  useEffect(() => {
    invoke<AppSettings>("system_get_settings")
      .then((loaded) => {
        setSettings(loaded);
        const resolved = THEME_OPTIONS.some((t) => t.id === loaded.theme_mode)
          ? (loaded.theme_mode as ThemeMode)
          : "dark";
        setTheme(resolved);
      })
      .catch(() => {});
    refreshVaultStatus();
  }, []);

  const handleThemeChange = (next: ThemeMode) => {
    setTheme(next);
    const updated = { ...settings, theme_mode: next };
    setSettings(updated);
    invoke("system_save_settings", { settings: updated }).catch(() => {});
  };

  const activeOwner = settings.github_org_name.trim() || settings.github_username.trim();
  const isOrgContext = Boolean(settings.github_org_name.trim());

  return (
    <div className={`app-root theme-${theme} d-flex flex-column`}>
      <Header
        activeRepo={activeRepoPath}
        activeOwner={activeOwner}
        isOrgContext={isOrgContext}
        vaultActive={vaultActive}
        theme={theme}
        onThemeChange={handleThemeChange}
      />
      <div className="d-flex flex-1 overflow-hidden">
        <Sidebar currentTab={currentTab} onSelectTab={setCurrentTab} />
        <main className="flex-1 overflow-auto p-4 bg-canvas-default">
          {currentTab === "upload" && (
            <UploadSyncView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
              settings={settings}
            />
          )}
          {currentTab === "graph" && <GitGraphView repoPath={activeRepoPath} />}
          {currentTab === "mirror" && (
            <MultiRemoteView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "merge" && (
            <ConflictResolverView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "semantic" && (
            <SemanticSearchView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "branches" && (
            <StaleBranchView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "repos" && <RepositoriesView settings={settings} />}
          {currentTab === "security" && (
            <SecurityScannerView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "pulls" && <PullRequestsView settings={settings} />}
          {currentTab === "actions" && <ActionsView settings={settings} />}
          {currentTab === "issues" && <IssuesReleasesView settings={settings} />}
          {currentTab === "ssh" && <SshKeysView />}
          {currentTab === "plugins" && (
            <PluginsView onNavigateToMarketplace={() => setCurrentTab("marketplace")} />
          )}
          {currentTab === "marketplace" && <MarketplaceView />}
          {currentTab === "analytics" && (
            <AnalyticsView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "schedule" && (
            <SyncScheduleSettingsView
              repoPath={activeRepoPath}
              onSelectRepoPath={setActiveRepoPath}
            />
          )}
          {currentTab === "enterprise" && <EnterpriseUpdatesView />}
          {currentTab === "report" && <TokenReportView />}
          {currentTab === "guide" && <UserGuideView />}
          {currentTab === "settings" && (
            <SettingsView
              settings={settings}
              onSettingsUpdated={(s) => {
                setSettings(s);
                setTheme(s.theme_mode);
              }}
              onVaultChange={refreshVaultStatus}
            />
          )}
        </main>
      </div>
      <StephenCopilotDrawer />
    </div>
  );
}
