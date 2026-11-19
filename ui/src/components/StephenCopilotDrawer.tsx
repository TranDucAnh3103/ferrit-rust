import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { SparklesFillIcon, XIcon, PaperAirplaneIcon } from "@primer/octicons-react";
import { formatIpcError } from "../types/ipc";

interface ChatTurn {
  role: "user" | "assistant";
  content: string;
}

export const StephenCopilotDrawer: React.FC = () => {
  const [open, setOpen] = useState(false);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [messages, setMessages] = useState<ChatTurn[]>([
    {
      role: "assistant",
      content:
        "Xin chào! Mình là Stephen Cloud AI. Mình chạy 100% qua Cloud AI SSE (<28MB RAM) — bạn có thể hỏi về Git, GitHub, hoặc nhờ chẩn đoán lỗi push.",
    },
  ]);

  useEffect(() => {
    const unlistenPromise = listen<{ stream_id: string; chunk: string; done: boolean }>(
      "ai-stream-chunk",
      (event) => {
        if (!event.payload.chunk) return;
        setMessages((prev) => {
          const copy = [...prev];
          const last = copy[copy.length - 1];
          if (last && last.role === "assistant") {
            copy[copy.length - 1] = { ...last, content: last.content + event.payload.chunk };
          }
          return copy;
        });
      }
    );
    return () => {
      unlistenPromise.then((u) => u());
    };
  }, []);

  const sendMessage = async () => {
    const trimmed = input.trim();
    if (!trimmed || streaming) return;

    const historyPayload = messages.slice(-10).map((m) => ({
      role: m.role,
      content: m.content,
    }));

    setMessages((prev) => [
      ...prev,
      { role: "user", content: trimmed },
      { role: "assistant", content: "" },
    ]);
    setInput("");
    setStreaming(true);

    try {
      const reply = await invoke<string>("ai_chat_send", {
        streamId: `stream-${Date.now()}`,
        history: historyPayload,
        userInput: trimmed,
      });
      setMessages((prev) => {
        const copy = [...prev];
        if (copy.length > 0 && copy[copy.length - 1].role === "assistant") {
          copy[copy.length - 1] = { role: "assistant", content: reply };
        }
        return copy;
      });
    } catch (err) {
      setMessages((prev) => {
        const copy = [...prev];
        if (copy.length > 0 && copy[copy.length - 1].role === "assistant") {
          copy[copy.length - 1] = {
            role: "assistant",
            content: `Lỗi Cloud AI: ${formatIpcError(err)}`,
          };
        }
        return copy;
      });
    } finally {
      setStreaming(false);
    }
  };

  return (
    <>
      <button
        className="btn btn-primary position-fixed d-flex flex-items-center shadow-large"
        style={{ right: 20, bottom: 20, zIndex: 50, borderRadius: "999px", padding: "10px 16px" }}
        onClick={() => setOpen((o) => !o)}
      >
        <SparklesFillIcon size={16} className="mr-2" />
        Stephen AI
      </button>

      {open && (
        <div
          className="primer-elevated position-fixed d-flex flex-column"
          style={{ right: 20, bottom: 72, width: 390, height: 490, zIndex: 50 }}
        >
          <div className="d-flex flex-items-center flex-justify-between p-3 border-bottom border-default">
            <div className="d-flex flex-items-center">
              <SparklesFillIcon size={16} className="mr-2 text-accent" />
              <span className="text-bold f5">Stephen Pure Cloud AI</span>
            </div>
            <button
              className="btn-octicon"
              aria-label="Close Stephen Cloud AI drawer"
              title="Close"
              onClick={() => setOpen(false)}
            >
              <XIcon size={16} />
            </button>
          </div>

          <div className="flex-1 overflow-auto p-3 d-flex flex-column gap-2">
            {messages.map((m, idx) => (
              <div
                key={idx}
                className={`p-2 rounded-2 f6 ${
                  m.role === "user"
                    ? "bg-accent-subtle text-primary ml-4"
                    : "bg-canvas-inset border border-muted text-primary mr-4"
                }`}
                style={{ whiteSpace: "pre-wrap" }}
              >
                <div className="text-bold text-muted mb-1">
                  {m.role === "user" ? "You" : "Stephen"}
                </div>
                {m.content || (streaming ? "..." : "")}
              </div>
            ))}
          </div>

          <div className="p-2 border-top border-default d-flex gap-2">
            <input
              type="text"
              className="primer-input flex-1"
              placeholder="Ask Stephen or run `push` / `clone <url>`..."
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") sendMessage();
              }}
            />
            <button
              className="btn btn-primary btn-sm"
              aria-label="Send message to Stephen Cloud AI"
              title="Send message"
              onClick={sendMessage}
              disabled={streaming}
            >
              <PaperAirplaneIcon size={14} />
            </button>
          </div>
        </div>
      )}
    </>
  );
};

export default StephenCopilotDrawer;
