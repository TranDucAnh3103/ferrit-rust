import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GitPullRequestIcon, GitMergeIcon, ArrowRightIcon, GitBranchIcon } from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import { AppSettings, PullRequestInfo, formatIpcError } from "../types/ipc";

export const PullRequestsView: React.FC<{ settings: AppSettings }> = ({ settings }) => {
  const { t } = useI18n();
  const [owner, setOwner] = useState(settings.github_org_name || settings.github_username);
  const [repo, setRepo] = useState("");
  const [prs, setPrs] = useState<PullRequestInfo[]>([]);
  const [title, setTitle] = useState("");
  const [head, setHead] = useState("feat/update");
  const [base, setBase] = useState("main");
  const [draft, setDraft] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const loadPrs = async () => {
    if (!owner || !repo) return;
    setErrorMsg(null);
    try {
      const list = await invoke<PullRequestInfo[]>("pr_list", {
        owner,
        repo,
        prState: "open",
        accountId: "default",
      });
      setPrs(list);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const createPr = async () => {
    if (!owner || !repo || !title) return;
    setErrorMsg(null);
    try {
      await invoke("pr_create", {
        owner,
        repo,
        req: { title, head, base, body: "Created via FERRIT Desktop v2.6", draft },
        accountId: "default",
      });
      setTitle("");
      await loadPrs();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const mergePr = async (prNumber: number, mergeMethod: string) => {
    try {
      await invoke("pr_merge", {
        owner,
        repo,
        prNumber,
        req: { commit_title: null, commit_message: null, merge_method: mergeMethod },
        accountId: "default",
      });
      await loadPrs();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="primer-surface p-3 d-flex gap-2 flex-wrap">
        <input className="primer-input" placeholder={t("pulls.owner")} value={owner} onChange={(e) => setOwner(e.target.value)} style={{ width: 180 }} />
        <input className="primer-input flex-1" placeholder={t("pulls.repo")} value={repo} onChange={(e) => setRepo(e.target.value)} />
        <button className="btn btn-primary d-inline-flex flex-items-center gap-1" onClick={loadPrs}>
          <GitPullRequestIcon size={14} /> {t("pulls.fetchPrs")}
        </button>
      </div>

      {errorMsg && <div className="flash flash-error p-2 f6">{errorMsg}</div>}

      <div className="primer-surface p-3">
        <h3 className="f5 text-bold mb-2">{t("pulls.createPr")}</h3>
        <div className="d-flex flex-wrap gap-2">
          <input className="primer-input flex-1" placeholder={t("pulls.prTitle")} value={title} onChange={(e) => setTitle(e.target.value)} />
          <input className="primer-input" placeholder={t("pulls.headBranch")} value={head} onChange={(e) => setHead(e.target.value)} style={{ width: 140 }} />
          <input className="primer-input" placeholder={t("pulls.baseBranch")} value={base} onChange={(e) => setBase(e.target.value)} style={{ width: 100 }} />
          <label className="d-flex flex-items-center f6 cursor-pointer">
            <input type="checkbox" className="mr-1" checked={draft} onChange={(e) => setDraft(e.target.checked)} />
            {t("pulls.draft")}
          </label>
          <button className="btn btn-primary btn-sm" onClick={createPr}>
            {t("pulls.createPr")}
          </button>
        </div>
      </div>

      <div className="primer-surface p-3">
        <h3 className="f5 text-bold mb-2">{t("pulls.title")} ({prs.length})</h3>
        {prs.length === 0 ? (
          <div className="empty-state-card text-center p-4">
            <div className="empty-state-icon-box">
              <GitPullRequestIcon size={24} className="text-accent" />
            </div>
            <div className="f4 text-bold">{t("pulls.noPrs")}</div>
            <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 440, margin: "8px auto" }}>
              {t("pulls.subtitle")}
            </p>
          </div>
        ) : (
          prs.map((pr) => (
            <div key={pr.id} className="interactive-row d-flex flex-items-center flex-justify-between p-2 border-bottom border-muted">
              <div>
                <span className="text-bold mr-2">
                  <span className="tabular-nums text-muted mr-1">#{pr.number}</span> {pr.title}
                </span>
                {pr.draft && <span className="Label mr-1">{t("pulls.draft")}</span>}
                <div className="text-small text-muted d-flex flex-items-center gap-1 mt-1">
                  <GitBranchIcon size={12} />
                  <span className="text-mono">{pr.head_ref}</span>
                  <ArrowRightIcon size={12} />
                  <span className="text-mono">{pr.base_ref}</span>
                  <span>by {pr.user_login}</span>
                </div>
              </div>
              <div className="d-flex gap-2">
                <button className="btn btn-sm btn-primary d-inline-flex flex-items-center gap-1" onClick={() => mergePr(pr.number, "squash")}>
                  <GitMergeIcon size={12} /> {t("pulls.squash")}
                </button>
                <button className="btn btn-sm btn-secondary" onClick={() => mergePr(pr.number, "rebase")}>
                  {t("pulls.rebase")}
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default PullRequestsView;
