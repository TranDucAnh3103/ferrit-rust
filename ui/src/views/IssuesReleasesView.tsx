import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IssueOpenedIcon, TagIcon, SparklesFillIcon } from "@primer/octicons-react";
import { AppSettings, IssueSummary, formatIpcError } from "../types/ipc";
import { useI18n } from "../i18n/I18nContext";

export const IssuesReleasesView: React.FC<{ settings: AppSettings }> = ({ settings }) => {
  const { t } = useI18n();
  const [owner, setOwner] = useState(settings.github_org_name || settings.github_username);
  const [repo, setRepo] = useState("");
  const [issues, setIssues] = useState<IssueSummary[]>([]);
  const [issueTitle, setIssueTitle] = useState("");
  const [issueBody, setIssueBody] = useState("");
  const [tagName, setTagName] = useState("v2.6.0");
  const [releaseTitle, setReleaseTitle] = useState("FERRIT v2.6.0");
  const [releaseBody, setReleaseBody] = useState("");
  const [statusMsg, setStatusMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadIssues = async () => {
    if (!owner || !repo) return;
    setErrorMsg(null);
    try {
      const list = await invoke<IssueSummary[]>("issues_list", { owner, repo, accountId: "default" });
      setIssues(list);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const createIssue = async () => {
    if (!owner || !repo || !issueTitle) return;
    setErrorMsg(null);
    try {
      await invoke("issues_create", {
        owner,
        repo,
        req: { title: issueTitle, body: issueBody || null, labels: [] },
        accountId: "default",
      });
      setIssueTitle("");
      setIssueBody("");
      await loadIssues();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const draftAiChangelog = async () => {
    setErrorMsg(null);
    try {
      const changelog = await invoke<string>("ai_generate_changelog", {
        tagName,
        commits: ["feat(core): add domain models", "feat(git): add interactive DAG revwalk"],
      });
      setReleaseBody(changelog);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const createRelease = async () => {
    if (!owner || !repo || !tagName) return;
    setErrorMsg(null);
    try {
      const releaseId = await invoke<number>("releases_create", {
        owner,
        repo,
        req: { tag_name: tagName, name: releaseTitle, body: releaseBody, draft: false, prerelease: false },
        accountId: "default",
      });
      setStatusMsg(`Published release #${releaseId} for tag ${tagName}`);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="primer-surface p-3 d-flex gap-2">
        <input className="primer-input" placeholder={t("pulls.owner")} value={owner} onChange={(e) => setOwner(e.target.value)} />
        <input className="primer-input flex-1" placeholder={t("pulls.repo")} value={repo} onChange={(e) => setRepo(e.target.value)} />
        <button className="btn btn-primary" onClick={loadIssues}>
          <IssueOpenedIcon size={14} className="mr-1" /> {t("issues.fetchIssues")}
        </button>
      </div>

      {errorMsg && <div className="flash flash-error p-2 f6">{errorMsg}</div>}
      {statusMsg && <div className="flash flash-success p-2 f6">{statusMsg}</div>}

      <div className="d-flex gap-3">
        {/* Left: Issues */}
        <div className="flex-1 primer-surface p-3">
          <h3 className="f5 text-bold mb-2">{t("issues.tabIssues")} ({issues.length})</h3>
          <div className="d-flex flex-column gap-2 mb-3">
            <input className="primer-input" placeholder={t("issues.issueTitle")} value={issueTitle} onChange={(e) => setIssueTitle(e.target.value)} />
            <textarea className="primer-input" rows={2} placeholder={t("issues.issueBody")} value={issueBody} onChange={(e) => setIssueBody(e.target.value)} />
            <button className="btn btn-sm btn-primary" onClick={createIssue}>
              {t("issues.createIssue")}
            </button>
          </div>
          {issues.length === 0 ? (
            <div className="empty-state-card text-center p-3">
              <div className="empty-state-icon-box">
                <IssueOpenedIcon size={20} className="text-muted" />
              </div>
              <div className="f5 text-bold">{t("issues.noIssues")}</div>
              <p className="f6 text-muted mt-1 mb-0">
                {t("issues.subtitle")}
              </p>
            </div>
          ) : (
            issues.map((issue) => (
              <div key={issue.id} className="interactive-row p-2 border-bottom border-muted f6">
                <span className="text-bold">
                  <span className="tabular-nums text-muted mr-1">#{issue.number}</span> {issue.title}
                </span>{" "}
                <span className={`Label ml-1 ${issue.state === "open" ? "Label--success" : "Label--secondary"}`}>
                  {issue.state}
                </span>
                <div className="text-muted">by {issue.author}</div>
              </div>
            ))
          )}
        </div>

        {/* Right: Release Publisher + AI Changelog */}
        <div className="flex-1 primer-surface p-3">
          <h3 className="f5 text-bold mb-2">
            <TagIcon size={15} className="mr-1" /> {t("issues.tabReleases")} &amp; {t("issues.aiDraftChangelog")}
          </h3>
          <div className="d-flex flex-column gap-2">
            <input className="primer-input" placeholder={t("issues.tagVersion")} value={tagName} onChange={(e) => setTagName(e.target.value)} />
            <input className="primer-input" placeholder={t("issues.releaseTitle")} value={releaseTitle} onChange={(e) => setReleaseTitle(e.target.value)} />
            <textarea className="primer-input" rows={6} placeholder={t("issues.releaseNotes")} value={releaseBody} onChange={(e) => setReleaseBody(e.target.value)} />
            <div className="d-flex gap-2">
              <button className="btn btn-sm btn-secondary d-inline-flex flex-items-center gap-1" onClick={draftAiChangelog}>
                <SparklesFillIcon size={12} /> {t("issues.aiDraftChangelog")}
              </button>
              <button className="btn btn-sm btn-primary" onClick={createRelease}>
                {t("issues.publishRelease")}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default IssuesReleasesView;
