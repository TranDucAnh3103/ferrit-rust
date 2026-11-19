import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SearchIcon, TrashIcon, FileDirectoryIcon, FileIcon, LinkExternalIcon, RepoIcon } from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import { AppSettings, ContentInfo, RepositoryInfo, formatIpcError } from "../types/ipc";

export const RepositoriesView: React.FC<{ settings: AppSettings }> = ({ settings }) => {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [sortBy, setSortBy] = useState("default");
  const [repos, setRepos] = useState<RepositoryInfo[]>([]);
  const [selectedRepo, setSelectedRepo] = useState<RepositoryInfo | null>(null);
  const [contents, setContents] = useState<ContentInfo[]>([]);
  const [currentPath, setCurrentPath] = useState("");
  const [readmeHtml, setReadmeHtml] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const owner = settings.github_org_name.trim() || settings.github_username.trim();
  const isOrg = Boolean(settings.github_org_name.trim());

  const runSearch = async () => {
    if (!owner) {
      setErrorMsg("Configure GitHub Username or Organization in Settings first.");
      return;
    }
    setErrorMsg(null);
    try {
      const res = await invoke<{ repos: RepositoryInfo[]; total_count: number }>("repo_search", {
        owner,
        query,
        isOrg,
        page: 1,
        pageSize: 20,
        sortBy,
        accountId: "default",
      });
      setRepos(res.repos);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const inspectRepo = async (repo: RepositoryInfo, path = "") => {
    setSelectedRepo(repo);
    setCurrentPath(path);
    try {
      const items = await invoke<ContentInfo[]>("repo_contents", {
        owner,
        repo: repo.name,
        path,
        accountId: "default",
      });
      setContents(items);
      if (path === "") {
        const readme = await invoke<string | null>("repo_readme", {
          owner,
          repo: repo.name,
          accountId: "default",
        });
        setReadmeHtml(readme);
      }
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  const deleteRepo = async (repoName: string) => {
    try {
      await invoke("repo_delete", { owner, repo: repoName, accountId: "default" });
      setSelectedRepo(null);
      await runSearch();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="primer-surface p-3 d-flex gap-2">
        <input
          type="text"
          className="primer-input flex-1"
          placeholder={`Search ${isOrg ? `org:${owner}` : `user:${owner}`} repositories...`}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") runSearch();
          }}
        />
        <select className="primer-input" value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
          <option value="default">{t("repos.sortUpdated")}</option>
          <option value="empty">{t("repos.sortCreated")}</option>
          <option value="new">{t("repos.sortPushed")}</option>
        </select>
        <button className="btn btn-primary" onClick={runSearch}>
          <SearchIcon size={14} className="mr-1" /> {t("common.search")}
        </button>
      </div>

      {errorMsg && <div className="flash flash-error p-2 f6">{errorMsg}</div>}

      <div className="d-flex gap-3">
        <div className="flex-1 primer-surface p-3">
          <h3 className="f5 text-bold mb-2">{t("repos.title")} ({repos.length})</h3>
          {repos.length === 0 ? (
            <div className="empty-state-card text-center p-4">
              <div className="empty-state-icon-box">
                <RepoIcon size={24} className="text-accent" />
              </div>
              <div className="f4 text-bold">{t("repos.noReposFound")}</div>
              <p className="f6 text-muted mt-2 mb-3">
                {t("repos.subtitle")}
              </p>
              <button className="btn btn-primary d-inline-flex flex-items-center gap-1" onClick={runSearch}>
                <SearchIcon size={14} /> {t("common.search")}
              </button>
            </div>
          ) : (
            repos.map((r) => {
              const isEmpty = r.size === 0 || !r.default_branch;
              return (
                <div
                  key={r.name}
                  onClick={() => inspectRepo(r, "")}
                  className="interactive-row d-flex flex-items-center flex-justify-between p-2 border-bottom border-muted"
                >
                  <div>
                    <span className="text-bold text-primary mr-2">{r.name}</span>
                    <span className="Label mr-1">{r.private ? "Private" : "Public"}</span>
                    {isEmpty && <span className="Label Label--attention mr-1">EMPTY</span>}
                  </div>
                  <div className="d-flex gap-2">
                    <button
                      className="btn btn-sm"
                      aria-label={`Open ${r.name} on GitHub`}
                      title="Open on GitHub"
                      onClick={(e) => {
                        e.stopPropagation();
                        invoke("system_open_browser", { url: r.html_url });
                      }}
                    >
                      <LinkExternalIcon size={12} />
                    </button>
                    <button
                      className="btn btn-sm btn-danger"
                      aria-label={`Delete repository ${r.name}`}
                      title="Delete repository"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteRepo(r.name);
                      }}
                    >
                      <TrashIcon size={12} />
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>

        {selectedRepo ? (
          <div className="flex-1 primer-surface p-3 overflow-auto" style={{ maxHeight: 540 }}>
            <h3 className="f5 text-bold mb-2">
              {selectedRepo.name} {currentPath ? `/ ${currentPath}` : ""}
            </h3>
            {currentPath && (
              <button
                className="btn btn-sm mb-2"
                onClick={() => {
                  const parts = currentPath.split("/");
                  parts.pop();
                  inspectRepo(selectedRepo, parts.join("/"));
                }}
              >
                .. (Parent)
              </button>
            )}
            <div className="Box Box--condensed mb-3">
              {contents.map((item) => (
                <div
                  key={item.path}
                  className="Box-row d-flex flex-items-center flex-justify-between f6 cursor-pointer"
                  onClick={() => item.type === "dir" && inspectRepo(selectedRepo, item.path)}
                >
                  <span className="d-flex flex-items-center">
                    {item.type === "dir" ? (
                      <FileDirectoryIcon size={14} className="mr-2 text-primary" />
                    ) : (
                      <FileIcon size={14} className="mr-2 text-muted" />
                    )}
                    {item.name}
                  </span>
                  <span className="text-muted text-mono">{item.type === "dir" ? "-" : `${item.size} B`}</span>
                </div>
              ))}
            </div>
            {readmeHtml && (
              <div className="Box p-3 f6">
                <div className="text-bold mb-2">README.md (Sanitized)</div>
                <div dangerouslySetInnerHTML={{ __html: readmeHtml }} />
              </div>
            )}
          </div>
        ) : (
          <div className="flex-1 primer-surface p-4 d-flex flex-justify-center flex-items-center">
            <div className="empty-state-card text-center" style={{ maxWidth: 380 }}>
              <div className="empty-state-icon-box">
                <FileDirectoryIcon size={24} className="text-muted" />
              </div>
              <div className="f4 text-bold">No Repository Selected</div>
              <p className="f6 text-muted mt-2">
                Click on any repository in the list on the left to browse its file tree, inspect contents, and view its sanitized README.
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default RepositoriesView;
