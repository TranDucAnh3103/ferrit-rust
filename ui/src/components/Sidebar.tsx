import React from "react";
import {
  UploadIcon,
  GitCommitIcon,
  GitMergeIcon,
  GitBranchIcon,
  SyncIcon,
  RepoIcon,
  GitPullRequestIcon,
  PlayIcon,
  IssueOpenedIcon,
  KeyIcon,
  ShieldCheckIcon,
  ShieldLockIcon,
  SearchIcon,
  ClockIcon,
  GraphIcon,
  PulseIcon,
  BookIcon,
  GearIcon,
  PlugIcon,
  PackageIcon,
} from "@primer/octicons-react";
import { Cpu } from "lucide-react";
import type { ActiveTab } from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

interface SidebarProps {
  currentTab: ActiveTab;
  onSelectTab: (tab: ActiveTab) => void;
}

interface NavGroupDef {
  headingKey: string;
  items: { id: ActiveTab; labelKey: string; icon: React.ReactNode }[];
}

const NAV_GROUPS_DEF: NavGroupDef[] = [
  {
    headingKey: "nav.group.gitWorkflow",
    items: [
      { id: "upload", labelKey: "nav.tab.upload", icon: <UploadIcon size={16} /> },
      { id: "graph", labelKey: "nav.tab.graph", icon: <GitCommitIcon size={16} /> },
      { id: "mirror", labelKey: "nav.tab.mirror", icon: <SyncIcon size={16} /> },
      { id: "merge", labelKey: "nav.tab.merge", icon: <GitMergeIcon size={16} /> },
      { id: "semantic", labelKey: "nav.tab.semantic", icon: <SearchIcon size={16} /> },
      { id: "branches", labelKey: "nav.tab.branches", icon: <GitBranchIcon size={16} /> },
      { id: "repos", labelKey: "nav.tab.repos", icon: <RepoIcon size={16} /> },
    ],
  },
  {
    headingKey: "nav.group.githubDevSecOps",
    items: [
      { id: "security", labelKey: "nav.tab.security", icon: <ShieldCheckIcon size={16} /> },
      { id: "pulls", labelKey: "nav.tab.pulls", icon: <GitPullRequestIcon size={16} /> },
      { id: "actions", labelKey: "nav.tab.actions", icon: <PlayIcon size={16} /> },
      { id: "issues", labelKey: "nav.tab.issues", icon: <IssueOpenedIcon size={16} /> },
      { id: "ssh", labelKey: "nav.tab.ssh", icon: <KeyIcon size={16} /> },
    ],
  },
  {
    headingKey: "nav.group.pluginsExtensibility",
    items: [
      { id: "plugins", labelKey: "nav.tab.plugins", icon: <PlugIcon size={16} /> },
      { id: "marketplace", labelKey: "nav.tab.marketplace", icon: <PackageIcon size={16} /> },
    ],
  },
  {
    headingKey: "nav.group.systemDiagnostics",
    items: [
      { id: "analytics", labelKey: "nav.tab.analytics", icon: <GraphIcon size={16} /> },
      { id: "schedule", labelKey: "nav.tab.schedule", icon: <ClockIcon size={16} /> },
      { id: "enterprise", labelKey: "nav.tab.enterprise", icon: <ShieldLockIcon size={16} /> },
      { id: "report", labelKey: "nav.tab.report", icon: <PulseIcon size={16} /> },
      { id: "guide", labelKey: "nav.tab.guide", icon: <BookIcon size={16} /> },
      { id: "settings", labelKey: "nav.tab.settings", icon: <GearIcon size={16} /> },
    ],
  },
];

export const Sidebar: React.FC<SidebarProps> = ({ currentTab, onSelectTab }) => {
  const { t } = useI18n();

  return (
    <aside
      className="bg-canvas-subtle border-right border-default d-flex flex-column p-2 flex-shrink-0"
      style={{ width: 260, minWidth: 260 }}
    >
      <div className="d-flex flex-items-center gap-2 px-2 py-2 mb-2 border-bottom border-muted">
        <img
          src="/icon.png"
          alt="FERRIT Icon"
          style={{ width: 22, height: 22, borderRadius: 4 }}
          className="flex-shrink-0"
        />
        <span className="text-bold text-primary f5">FERRIT</span>
        <span className="Label Label--secondary ml-auto" style={{ fontSize: 10 }}>Rust 2.6</span>
      </div>

      <nav className="flex-1 overflow-auto" aria-label="Primary Sidebar">
        {NAV_GROUPS_DEF.map((group) => (
          <div key={group.headingKey} className="mb-3">
            <div
              className="px-2 py-1 text-small text-bold text-muted"
              style={{ letterSpacing: "0.04em", textTransform: "uppercase", fontSize: 11 }}
            >
              {t(group.headingKey)}
            </div>
            <div className="d-flex flex-column gap-1">
              {group.items.map((item) => {
                const active = currentTab === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    aria-current={active ? "page" : undefined}
                    onClick={() => onSelectTab(item.id)}
                    className={`sidebar-nav-item ${active ? "is-active" : ""}`}
                    title={t(item.labelKey)}
                  >
                    <span className="d-flex flex-items-center flex-shrink-0" style={{ width: 16 }}>
                      {item.icon}
                    </span>
                    <span className="text-truncate flex-1" style={{ whiteSpace: "nowrap" }}>
                      {t(item.labelKey)}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      <div className="p-2 mt-auto rounded-2 bg-canvas-inset border border-muted text-small text-muted d-flex flex-items-center gap-2">
        <Cpu size={14} className="text-success flex-shrink-0" />
        <div className="text-truncate">
          <div className="text-bold text-primary f6">{t("app.pureRust")}</div>
          <div style={{ fontSize: 11 }}>{t("app.lowRam")}</div>
        </div>
      </div>
    </aside>
  );
};

export default Sidebar;
