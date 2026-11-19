import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  SearchIcon,
  GitCommitIcon,
  FileDirectoryIcon,
  DatabaseIcon,
  CheckIcon,
  CopyIcon,
} from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import { formatIpcError, type SearchHit } from "../types/ipc";

interface SemanticSearchViewProps {
  repoPath: string;
  onSelectRepoPath: (path: string) => void;
}

export const SemanticSearchView: React.FC<SemanticSearchViewProps> = ({
  repoPath,
  onSelectRepoPath,
}) => {
  const { t } = useI18n();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [indexedCount, setIndexedCount] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [copiedOid, setCopiedOid] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const pickFolder = async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string" && selected.trim()) {
      onSelectRepoPath(selected);
    }
  };

  const handleIndexCommits = async () => {
    if (!repoPath.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      const count = await invoke<number>("ai_index_commits", {
        repoPath,
        maxCommits: 250,
      });
      setIndexedCount(count);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!repoPath.trim() || !query.trim()) return;
    setBusy(true);
    setErrorMsg(null);
    try {
      const hits = await invoke<SearchHit[]>("ai_semantic_search", {
        repoPath,
        query: query.trim(),
        topK: 20,
      });
      setResults(hits);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    } finally {
      setBusy(false);
    }
  };

  const copyOid = async (oid: string) => {
    try {
      await navigator.clipboard.writeText(oid);
      setCopiedOid(oid);
      setTimeout(() => setCopiedOid(null), 1500);
    } catch {
      /* ignore clipboard refusal */
    }
  };

  return (
    <div className="d-flex flex-column gap-3">
      <div className="d-flex flex-justify-between flex-items-center flex-wrap gap-2">
        <div>
          <h2 className="f3 text-bold d-flex flex-items-center gap-2 m-0">
            <SearchIcon size={20} className="text-accent" />
            {t("semantic.title")}
          </h2>
          <p className="text-muted f6 m-0 mt-1">
            {t("semantic.subtitle")}
          </p>
        </div>

        <div className="d-flex flex-items-center gap-2">
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            onClick={pickFolder}
          >
            <FileDirectoryIcon size={14} />
            {t("common.browse")}
          </button>
          <button
            type="button"
            className="btn btn-secondary d-inline-flex flex-items-center gap-1"
            disabled={busy || !repoPath.trim()}
            onClick={handleIndexCommits}
          >
            <DatabaseIcon size={14} />
            {busy ? t("common.loading") : t("semantic.indexBtn")}
          </button>
        </div>
      </div>

      <form onSubmit={handleSearch} className="primer-card p-3 d-flex flex-column gap-2">
        <div className="d-flex gap-2 flex-wrap">
          <input
            type="text"
            className="form-control flex-1 f6 text-mono"
            placeholder="Repository path..."
            value={repoPath}
            onChange={(e) => onSelectRepoPath(e.target.value)}
          />
        </div>
        <div className="d-flex gap-2 flex-wrap">
          <input
            type="text"
            className="form-control flex-1 f5"
            placeholder={t("semantic.queryPlaceholder")}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <button
            type="submit"
            className="btn btn-primary d-inline-flex flex-items-center gap-1"
            disabled={busy || !repoPath.trim()}
          >
            <SearchIcon size={14} />
            {busy ? t("common.loading") : t("common.search")}
          </button>
        </div>
      </form>

      {indexedCount !== null && (
        <div
          className="primer-card p-2 text-success f6 tabular-nums"
          style={{ borderColor: "var(--color-success-fg)" }}
        >
          Indexed {indexedCount} new commit embedding(s) into SQLite vector cache.
        </div>
      )}

      {errorMsg && (
        <div
          className="primer-card p-2 text-danger f6"
          style={{ borderColor: "var(--color-danger-fg)" }}
        >
          {errorMsg}
        </div>
      )}

      {results.length === 0 ? (
        <div className="empty-state-card text-center p-4">
          <div className="empty-state-icon-box">
            <SearchIcon size={24} className="text-accent" />
          </div>
          <div className="f4 text-bold">{t("semantic.title")}</div>
          <p className="f6 text-muted mt-2 mb-3" style={{ maxWidth: 480, margin: "8px auto" }}>
            {t("semantic.subtitle")}
          </p>
          <div className="d-flex flex-justify-center gap-2 mt-2">
            <button
              type="button"
              className="btn btn-secondary d-inline-flex flex-items-center gap-1"
              disabled={busy || !repoPath.trim()}
              onClick={handleIndexCommits}
            >
              <DatabaseIcon size={14} />
              {t("semantic.indexBtn")}
            </button>
          </div>
        </div>
      ) : (
        <div className="d-flex flex-column gap-2">
          {results.map((hit) => (
            <div
              key={hit.commit_oid}
              className="primer-card p-3 d-flex flex-justify-between flex-items-center gap-3 interactive-row"
            >
              <div className="d-flex flex-items-center gap-2 min-width-0">
                <GitCommitIcon size={16} className="text-accent flex-shrink-0" />
                <div className="min-width-0">
                  <div className="f5 text-bold text-truncate">
                    {hit.summary}
                  </div>
                  <div className="text-mono f6 text-muted">
                    {hit.commit_oid}
                  </div>
                </div>
              </div>

              <div className="d-flex flex-items-center gap-2 flex-shrink-0">
                <span className="badge text-success tabular-nums">
                  {(hit.similarity_score * 100).toFixed(1)}% match
                </span>
                <button
                  type="button"
                  className="btn btn-secondary f6 d-inline-flex flex-items-center gap-1"
                  aria-label={`Copy commit OID ${hit.commit_oid.slice(0, 8)}`}
                  onClick={() => copyOid(hit.commit_oid)}
                >
                  {copiedOid === hit.commit_oid ? (
                    <CheckIcon size={13} className="text-success" />
                  ) : (
                    <CopyIcon size={13} />
                  )}
                  {copiedOid === hit.commit_oid ? t("common.copied") : "Copy SHA"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default SemanticSearchView;
