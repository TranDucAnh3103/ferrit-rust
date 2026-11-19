import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  WebhookDeliveryRecord,
  WebhookEndpoint,
  WebhookTransport,
} from "../../types/ipc";
import {
  BroadcastIcon,
  PlusIcon,
  TrashIcon,
  PaperAirplaneIcon,
  CheckCircleIcon,
  XCircleIcon,
  ClockIcon,
  ShieldIcon,
  ChevronDownIcon,
  ChevronUpIcon,
} from "@primer/octicons-react";

export const WebhooksSettings: React.FC = () => {
  const [endpoints, setEndpoints] = useState<WebhookEndpoint[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [showAddForm, setShowAddForm] = useState<boolean>(false);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [pingResult, setPingResult] = useState<{ id: string; status: number } | null>(null);
  const [expandedDeliveryId, setExpandedDeliveryId] = useState<string | null>(null);
  const [deliveries, setDeliveries] = useState<Record<string, WebhookDeliveryRecord[]>>({});

  // Form State
  const [name, setName] = useState<string>("");
  const [transport, setTransport] = useState<WebhookTransport>("discord");
  const [targetUrl, setTargetUrl] = useState<string>("");
  const [secret, setSecret] = useState<string>("");
  const [selectedEvents, setSelectedEvents] = useState<string[]>([
    "git_push",
    "git_sync_failed",
    "pr_created",
  ]);

  const loadEndpoints = async () => {
    setLoading(true);
    try {
      const res = await invoke<WebhookEndpoint[]>("webhook_list_endpoints");
      setEndpoints(res);
    } catch (err) {
      console.error("Failed to load webhooks:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadEndpoints();
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !targetUrl.trim()) {
      alert("Name and Target URL are required");
      return;
    }

    const newEndpoint: WebhookEndpoint = {
      id: "ep_" + Date.now().toString(36),
      name: name.trim(),
      transport,
      target_url: targetUrl.trim(),
      vault_secret_key: secret.trim() ? `webhook_sec_${Date.now().toString(36)}` : null,
      events: selectedEvents,
      is_active: true,
      created_at: new Date().toISOString(),
    };

    try {
      await invoke("webhook_upsert_endpoint", {
        endpoint: newEndpoint,
        secret: secret.trim() || null,
      });
      setName("");
      setTargetUrl("");
      setSecret("");
      setShowAddForm(false);
      await loadEndpoints();
    } catch (err) {
      alert(`Failed to save webhook: ${err}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Are you sure you want to remove this webhook endpoint?")) return;
    try {
      await invoke("webhook_delete_endpoint", { id });
      setEndpoints((prev) => prev.filter((ep) => ep.id !== id));
    } catch (err) {
      alert(`Delete failed: ${err}`);
    }
  };

  const handleTestPing = async (id: string) => {
    setTestingId(id);
    setPingResult(null);
    try {
      const status = await invoke<number>("webhook_test_ping", { endpointId: id });
      setPingResult({ id, status });
    } catch (err: any) {
      alert(`Test ping failed: ${typeof err === "string" ? err : err.message || "Network error"}`);
    } finally {
      setTestingId(null);
    }
  };

  const handleToggleDeliveries = async (endpointId: string) => {
    if (expandedDeliveryId === endpointId) {
      setExpandedDeliveryId(null);
      return;
    }

    setExpandedDeliveryId(endpointId);
    try {
      const res = await invoke<WebhookDeliveryRecord[]>("webhook_list_deliveries", {
        endpointId,
        limit: 10,
      });
      setDeliveries((prev) => ({ ...prev, [endpointId]: res }));
    } catch (err) {
      console.error("Failed to load deliveries:", err);
    }
  };

  const toggleEvent = (ev: string) => {
    setSelectedEvents((prev) =>
      prev.includes(ev) ? prev.filter((e) => e !== ev) : [...prev, ev]
    );
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <h3 style={{ margin: 0, fontSize: 16, fontWeight: 600, display: "flex", alignItems: "center", gap: 8 }}>
            <BroadcastIcon size={18} />
            Multi-Channel Webhooks & Dispatcher
          </h3>
          <p style={{ margin: "4px 0 0", fontSize: 12, color: "var(--color-fg-muted)" }}>
            Automatically dispatch push events, sync failures, and PRs to Discord, Slack, Telegram, or custom endpoints.
          </p>
        </div>

        <button
          onClick={() => setShowAddForm(!showAddForm)}
          style={{
            padding: "5px 12px",
            borderRadius: 6,
            border: "1px solid var(--color-btn-primary-border)",
            backgroundColor: "var(--color-btn-primary-bg)",
            color: "var(--color-btn-primary-text)",
            cursor: "pointer",
            fontSize: 12,
            fontWeight: 600,
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
          }}
        >
          <PlusIcon size={14} />
          {showAddForm ? "Cancel" : "Add Endpoint"}
        </button>
      </div>

      {/* Add Webhook Form */}
      {showAddForm && (
        <form
          onSubmit={handleSave}
          style={{
            backgroundColor: "var(--color-canvas-subtle)",
            border: "1px solid var(--color-border-default)",
            borderRadius: 8,
            padding: 16,
            display: "flex",
            flexDirection: "column",
            gap: 12,
          }}
        >
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
            <div>
              <label style={{ display: "block", fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
                Endpoint Name
              </label>
              <input
                type="text"
                placeholder="e.g. #dev-alerts Discord"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
                style={{
                  width: "100%",
                  padding: "6px 10px",
                  borderRadius: 6,
                  border: "1px solid var(--color-border-default)",
                  backgroundColor: "var(--color-canvas-default)",
                  color: "var(--color-fg-default)",
                  fontSize: 13,
                }}
              />
            </div>

            <div>
              <label style={{ display: "block", fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
                Platform / Transport
              </label>
              <select
                value={transport}
                onChange={(e) => setTransport(e.target.value as WebhookTransport)}
                style={{
                  width: "100%",
                  padding: "6px 10px",
                  borderRadius: 6,
                  border: "1px solid var(--color-border-default)",
                  backgroundColor: "var(--color-canvas-default)",
                  color: "var(--color-fg-default)",
                  fontSize: 13,
                }}
              >
                <option value="discord">Discord (Rich Embeds)</option>
                <option value="slack">Slack (Block Kit)</option>
                <option value="telegram">Telegram (MarkdownV2)</option>
                <option value="generic">Generic HTTP (HMAC-SHA256)</option>
              </select>
            </div>
          </div>

          <div>
            <label style={{ display: "block", fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
              Target Webhook URL
            </label>
            <input
              type="url"
              placeholder="https://discord.com/api/webhooks/... or https://api.slack.com/..."
              value={targetUrl}
              onChange={(e) => setTargetUrl(e.target.value)}
              required
              style={{
                width: "100%",
                padding: "6px 10px",
                borderRadius: 6,
                border: "1px solid var(--color-border-default)",
                backgroundColor: "var(--color-canvas-default)",
                color: "var(--color-fg-default)",
                fontSize: 13,
              }}
            />
          </div>

          {transport === "generic" && (
            <div>
              <label style={{ display: "block", fontSize: 12, fontWeight: 600, marginBottom: 4 }}>
                HMAC-SHA256 Signing Secret (Saved to OS Vault)
              </label>
              <input
                type="password"
                placeholder="Optional pre-shared secret for X-Hub-Signature-256 header"
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                style={{
                  width: "100%",
                  padding: "6px 10px",
                  borderRadius: 6,
                  border: "1px solid var(--color-border-default)",
                  backgroundColor: "var(--color-canvas-default)",
                  color: "var(--color-fg-default)",
                  fontSize: 13,
                }}
              />
            </div>
          )}

          <div>
            <label style={{ display: "block", fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
              Subscribed Event Types
            </label>
            <div style={{ display: "flex", gap: 14, flexWrap: "wrap", fontSize: 13 }}>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={selectedEvents.includes("git_push")}
                  onChange={() => toggleEvent("git_push")}
                />
                Git Push
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={selectedEvents.includes("git_sync_failed")}
                  onChange={() => toggleEvent("git_sync_failed")}
                />
                Sync Failed
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={selectedEvents.includes("pr_created")}
                  onChange={() => toggleEvent("pr_created")}
                />
                PR Created
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                <input
                  type="checkbox"
                  checked={selectedEvents.includes("batch_completed")}
                  onChange={() => toggleEvent("batch_completed")}
                />
                Batch Completed
              </label>
            </div>
          </div>

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 6 }}>
            <button
              type="button"
              onClick={() => setShowAddForm(false)}
              style={{
                padding: "6px 12px",
                borderRadius: 6,
                border: "1px solid var(--color-btn-border)",
                backgroundColor: "var(--color-btn-bg)",
                color: "var(--color-btn-text)",
                cursor: "pointer",
                fontSize: 12,
              }}
            >
              Cancel
            </button>
            <button
              type="submit"
              style={{
                padding: "6px 16px",
                borderRadius: 6,
                border: "1px solid var(--color-btn-primary-border)",
                backgroundColor: "var(--color-btn-primary-bg)",
                color: "var(--color-btn-primary-text)",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: 600,
              }}
            >
              Save Webhook Endpoint
            </button>
          </div>
        </form>
      )}

      {/* List of Endpoints */}
      {endpoints.length === 0 && !loading ? (
        <div
          style={{
            textAlign: "center",
            padding: 24,
            border: "1px dashed var(--color-border-default)",
            borderRadius: 6,
            color: "var(--color-fg-muted)",
            fontSize: 13,
          }}
        >
          No webhook endpoints configured yet. Click "Add Endpoint" to create one.
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 10 }}>
          {endpoints.map((ep) => (
            <div
              key={ep.id}
              style={{
                backgroundColor: "var(--color-canvas-default)",
                border: "1px solid var(--color-border-default)",
                borderRadius: 6,
                padding: 12,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <span style={{ fontWeight: 600, fontSize: 14 }}>{ep.name}</span>
                    <span
                      style={{
                        fontSize: 11,
                        padding: "1px 6px",
                        borderRadius: 10,
                        backgroundColor: "var(--color-accent-subtle)",
                        color: "var(--color-accent-fg)",
                        textTransform: "capitalize",
                      }}
                    >
                      {ep.transport}
                    </span>
                    {ep.vault_secret_key && (
                      <span
                        style={{
                          fontSize: 10,
                          padding: "1px 6px",
                          borderRadius: 10,
                          backgroundColor: "var(--color-success-subtle)",
                          color: "var(--color-success-fg)",
                          display: "inline-flex",
                          alignItems: "center",
                          gap: 3,
                        }}
                      >
                        <ShieldIcon size={10} />
                        Signed
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: 11, color: "var(--color-fg-muted)", marginTop: 2 }}>
                    {ep.target_url.slice(0, 55)}... · Events: {ep.events.join(", ")}
                  </div>
                </div>

                <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                  <button
                    onClick={() => handleTestPing(ep.id)}
                    disabled={testingId === ep.id}
                    style={{
                      padding: "4px 10px",
                      borderRadius: 4,
                      border: "1px solid var(--color-border-default)",
                      backgroundColor: "var(--color-canvas-subtle)",
                      color: "var(--color-fg-default)",
                      cursor: testingId === ep.id ? "not-allowed" : "pointer",
                      fontSize: 11,
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                    }}
                  >
                    <PaperAirplaneIcon size={12} />
                    {testingId === ep.id ? "Pinging..." : "Test Ping"}
                  </button>

                  <button
                    onClick={() => handleToggleDeliveries(ep.id)}
                    style={{
                      padding: "4px 8px",
                      borderRadius: 4,
                      border: "1px solid var(--color-border-default)",
                      backgroundColor: "transparent",
                      color: "var(--color-fg-muted)",
                      cursor: "pointer",
                      fontSize: 11,
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 4,
                    }}
                  >
                    <ClockIcon size={12} />
                    Logs
                    {expandedDeliveryId === ep.id ? <ChevronUpIcon size={12} /> : <ChevronDownIcon size={12} />}
                  </button>

                  <button
                    onClick={() => handleDelete(ep.id)}
                    style={{
                      padding: "4px 8px",
                      borderRadius: 4,
                      border: "1px solid var(--color-border-default)",
                      backgroundColor: "transparent",
                      color: "var(--color-danger-fg)",
                      cursor: "pointer",
                    }}
                  >
                    <TrashIcon size={12} />
                  </button>
                </div>
              </div>

              {pingResult && pingResult.id === ep.id && (
                <div
                  style={{
                    marginTop: 8,
                    fontSize: 11,
                    color: "var(--color-success-fg)",
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                  }}
                >
                  <CheckCircleIcon size={12} />
                  Ping successful! Target server returned HTTP {pingResult.status}
                </div>
              )}

              {/* Delivery Logs Expanded */}
              {expandedDeliveryId === ep.id && (
                <div
                  style={{
                    marginTop: 10,
                    paddingTop: 10,
                    borderTop: "1px solid var(--color-border-subtle)",
                  }}
                >
                  <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 6 }}>
                    Recent Delivery Audit Logs:
                  </div>
                  {deliveries[ep.id]?.length === 0 || !deliveries[ep.id] ? (
                    <div style={{ fontSize: 11, color: "var(--color-fg-muted)" }}>
                      No recent deliveries recorded for this endpoint.
                    </div>
                  ) : (
                    <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                      {deliveries[ep.id].map((d) => (
                        <div
                          key={d.id}
                          style={{
                            fontSize: 11,
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            backgroundColor: "var(--color-canvas-subtle)",
                            padding: "4px 8px",
                            borderRadius: 4,
                          }}
                        >
                          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                            <span style={{ color: d.status_code && d.status_code < 400 ? "var(--color-success-fg)" : "var(--color-danger-fg)" }}>
                              {d.status_code && d.status_code < 400 ? <CheckCircleIcon size={12} /> : <XCircleIcon size={12} />}
                            </span>
                            <span>{d.event_type}</span>
                            <span style={{ color: "var(--color-fg-muted)" }}>
                              (HTTP {d.status_code ?? "ERR"}, {d.duration_ms}ms, {d.attempt_count} attempt(s))
                            </span>
                          </div>
                          <span style={{ color: "var(--color-fg-muted)" }}>
                            {new Date(d.delivered_at).toLocaleTimeString()}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
