import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GitCommitIcon, VerifiedIcon, SyncIcon } from "@primer/octicons-react";
import { useI18n } from "../i18n/I18nContext";
import { GitDagResponse, GitGraphNode, formatIpcError } from "../types/ipc";

const ROW_HEIGHT = 36;
const COL_WIDTH = 16;
const START_X = 20;
const OVERSCAN_ROWS = 15;

export const GitGraphView: React.FC<{ repoPath: string }> = ({ repoPath }) => {
  const { t } = useI18n();
  const [commits, setCommits] = useState<GitGraphNode[]>([]);
  const [selectedCommit, setSelectedCommit] = useState<GitGraphNode | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportHeight, setViewportHeight] = useState(600);
  const scrollContainerRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  const loadGraph = async () => {
    if (!repoPath) return;
    setLoading(true);
    setErrorMsg(null);
    try {
      const dag = await invoke<GitDagResponse>("graph_get_commits", {
        path: repoPath,
        limit: 5000,
      });
      setCommits(dag.nodes);
      setSelectedCommit(dag.nodes[0] ?? null);
    } catch (err) {
      setErrorMsg(formatIpcError(err));
      setCommits([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadGraph();
  }, [repoPath]);

  const { startIndex, endIndex, visibleNodes } = useMemo(() => {
    const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS);
    const visibleCount = Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN_ROWS * 2;
    const end = Math.min(commits.length, start + visibleCount);
    return {
      startIndex: start,
      endIndex: end,
      visibleNodes: commits.slice(start, end),
    };
  }, [commits, scrollTop, viewportHeight]);

  // Draw ONLY visible viewport rows on Canvas for 60 FPS scrolling at 100k+ commits
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const width = 140;
    const height = viewportHeight;
    if (canvas.width !== width * dpr || canvas.height !== height * dpr) {
      canvas.width = width * dpr;
      canvas.height = height * dpr;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);

    if (commits.length === 0) return;

    // 1. Draw parent-child Bezier edges visible in viewport
    for (let i = startIndex; i < endIndex; i++) {
      const node = commits[i];
      const y1 = i * ROW_HEIGHT + ROW_HEIGHT / 2 - scrollTop;

      for (const edge of node.lines) {
        const x1 = START_X + edge.from_col * COL_WIDTH;
        const x2 = START_X + edge.to_col * COL_WIDTH;
        const y2 = edge.to_row * ROW_HEIGHT + ROW_HEIGHT / 2 - scrollTop;

        ctx.beginPath();
        ctx.moveTo(x1, y1);
        if (edge.from_col === edge.to_col) {
          ctx.lineTo(x2, y2);
        } else {
          const midY = (y1 + y2) / 2;
          ctx.bezierCurveTo(x1, midY, x2, midY, x2, y2);
        }
        ctx.strokeStyle = edge.color || "#1f6feb";
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    }

    // Resolve active canvas theme color dynamically so node outlines stay crisp in Dark & Light modes
    const appRoot = canvas.closest(".app-root");
    const nodeStroke =
      (appRoot ? getComputedStyle(appRoot).getPropertyValue("--color-canvas-default").trim() : "") ||
      "#0d1117";

    // 2. Draw commit circles on top of edges
    for (let i = startIndex; i < endIndex; i++) {
      const node = commits[i];
      const y = i * ROW_HEIGHT + ROW_HEIGHT / 2 - scrollTop;
      const x = START_X + node.column * COL_WIDTH;

      ctx.beginPath();
      ctx.arc(x, y, node.is_merge ? 6 : 5, 0, 2 * Math.PI);
      ctx.fillStyle = node.color || "#1f6feb";
      ctx.fill();
      ctx.strokeStyle = nodeStroke;
      ctx.lineWidth = 1.75;
      ctx.stroke();
    }
  }, [commits, startIndex, endIndex, scrollTop, viewportHeight]);

  const handleScroll = () => {
    const el = scrollContainerRef.current;
    if (!el) return;
    setScrollTop(el.scrollTop);
    setViewportHeight(el.clientHeight || 600);
  };

  const runAction = async (cmd: string, args: Record<string, unknown>) => {
    try {
      await invoke(cmd, args);
      await loadGraph();
    } catch (err) {
      setErrorMsg(formatIpcError(err));
    }
  };

  return (
    <div className="d-flex height-full border border-default rounded-2 overflow-hidden bg-canvas-default">
      {/* Left: Virtualized DAG Commit List */}
      <div className="flex-1 d-flex flex-column border-right border-default overflow-hidden">
        <div className="d-flex flex-items-center flex-justify-between p-3 border-bottom border-default">
          <div>
            <h2 className="f4 text-bold m-0">{t("graph.title")}</h2>
            <span className="text-small text-muted">
              Virtualized 60 FPS canvas ({commits.length} commits loaded)
            </span>
          </div>
          <button className="btn btn-sm btn-secondary d-inline-flex flex-items-center gap-1" onClick={loadGraph} disabled={loading || !repoPath}>
            <SyncIcon size={14} /> {loading ? t("common.loading") : t("common.refresh")}
          </button>
        </div>

        {errorMsg && <div className="flash flash-error m-2 p-2 f6">{errorMsg}</div>}

        <div
          ref={scrollContainerRef}
          onScroll={handleScroll}
          className="flex-1 overflow-auto position-relative d-flex"
        >
          {commits.length === 0 ? (
            <div className="flex-1 d-flex flex-justify-center flex-items-center p-4">
              <div className="empty-state-card text-center" style={{ maxWidth: 460 }}>
                <div className="empty-state-icon-box">
                  <GitCommitIcon size={24} className="text-accent" />
                </div>
                <div className="f4 text-bold">{t("graph.title")}</div>
                <p className="f6 text-muted mt-2 mb-3">
                  {repoPath
                    ? "No commits found on the active branch or repo is still scanning."
                    : "Please select or open a Git repository in Upload & Sync to inspect the commit graph."}
                </p>
                <button
                  type="button"
                  className="btn btn-primary d-inline-flex flex-items-center gap-1"
                  disabled={loading || !repoPath.trim()}
                  onClick={loadGraph}
                >
                  <SyncIcon size={14} />
                  {loading ? t("common.loading") : t("common.refresh")}
                </button>
              </div>
            </div>
          ) : (
            <>
              <canvas
                ref={canvasRef}
                className="position-sticky top-0 left-0 flex-shrink-0"
                style={{ width: 140, height: viewportHeight, pointerEvents: "none" }}
              />
              <div
                className="flex-1 position-relative"
                style={{ height: Math.max(commits.length * ROW_HEIGHT, 200) }}
              >
                {visibleNodes.map((c, offset) => {
                  const rowIdx = startIndex + offset;
                  return (
                    <div
                      key={c.oid}
                      onClick={() => setSelectedCommit(c)}
                      className={`position-absolute left-0 right-0 d-flex flex-items-center px-2 border-bottom border-muted text-small cursor-pointer ${
                        selectedCommit?.oid === c.oid ? "bg-accent-subtle" : ""
                      }`}
                      style={{ top: rowIdx * ROW_HEIGHT, height: ROW_HEIGHT }}
                    >
                      <span className="text-mono mr-2 text-muted f6">{c.short_sha}</span>
                      <span className="text-bold mr-2 text-truncate" style={{ maxWidth: 320 }}>
                        {c.summary}
                      </span>
                      {c.refs.map((r) => (
                        <span
                          key={`${r.ref_type}-${r.name}`}
                          className={`Label mr-1 ${r.is_head ? "Label--success" : "Label--accent"}`}
                        >
                          {r.name}
                        </span>
                      ))}
                      <span className="ml-auto text-muted f6">{c.author_name}</span>
                    </div>
                  );
                })}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Right: Commit Inspector Pane */}
      <div className="flex-shrink-0 p-4 bg-canvas-subtle overflow-auto" style={{ width: 380 }}>
        {selectedCommit ? (
          <div>
            <div className="d-flex flex-items-center mb-3">
              <GitCommitIcon size={20} className="mr-2 text-primary" />
              <span className="text-bold f4">{t("graph.commitDetails")}</span>
              {selectedCommit.signature_verified && (
                <span className="Label Label--success ml-auto d-flex flex-items-center">
                  <VerifiedIcon size={12} className="mr-1" /> {t("graph.verified")}
                </span>
              )}
            </div>

            <div className="Box Box--condensed mb-3 p-3">
              <div className="text-mono f6 text-muted mb-1">SHA: {selectedCommit.oid}</div>
              <div className="text-bold mb-2">{selectedCommit.summary}</div>
              {selectedCommit.body && (
                <pre className="f6 text-muted mb-2" style={{ whiteSpace: "pre-wrap" }}>
                  {selectedCommit.body}
                </pre>
              )}
              <div className="text-small text-muted">
                Author: {selectedCommit.author_name} &lt;{selectedCommit.author_email}&gt;
              </div>
            </div>

            <div className="d-flex flex-column gap-2">
              <button
                className="btn btn-secondary d-flex flex-items-center flex-justify-center gap-2"
                onClick={() => runAction("graph_checkout", { path: repoPath, targetRef: selectedCommit.oid })}
              >
                {t("graph.checkoutDetached")}
              </button>
              <button
                className="btn btn-secondary d-flex flex-items-center flex-justify-center gap-2"
                onClick={() => runAction("graph_cherry_pick", { path: repoPath, oid: selectedCommit.oid })}
              >
                {t("graph.cherryPick")}
              </button>
              <button
                className="btn btn-secondary d-flex flex-items-center flex-justify-center gap-2"
                onClick={() => runAction("graph_revert", { path: repoPath, oid: selectedCommit.oid })}
              >
                {t("graph.revert")}
              </button>
              <button
                className="btn btn-danger d-flex flex-items-center flex-justify-center gap-2"
                onClick={() => runAction("graph_reset", { path: repoPath, oid: selectedCommit.oid, mode: "Soft" })}
              >
                {t("graph.reset")} (Soft)
              </button>
            </div>
          </div>
        ) : (
          <div className="empty-state-card text-center p-4">
            <div className="empty-state-icon-box">
              <GitCommitIcon size={24} className="text-muted" />
            </div>
            <div className="f4 text-bold">No Commit Selected</div>
            <p className="f6 text-muted mt-2">
              Select any commit node in the DAG graph on the left to inspect its commit message, author, GPG verification, and perform checkout, cherry-pick, or revert.
            </p>
          </div>
        )}
      </div>
    </div>
  );
};

export default GitGraphView;
