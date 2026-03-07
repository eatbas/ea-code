import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import type { PipelineRun, PipelineStatus } from "../types";

interface ChatViewProps {
  run: PipelineRun;
  logs: string[];
  artifacts: Record<string, string>;
  onCancel: () => void;
  onBackToHome: () => void;
  onOpenSettings: () => void;
}

/** Whether the pipeline is actively executing. */
function isActive(status: PipelineStatus): boolean {
  return status === "running" || status === "waiting_for_input";
}

/** Whether the pipeline is in a terminal state. */
function isTerminal(status: PipelineStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

/** Status label and colour for the current pipeline state. */
function statusInfo(status: PipelineStatus): { label: string; colour: string } {
  switch (status) {
    case "running":
      return { label: "Running", colour: "#6366f1" };
    case "waiting_for_input":
      return { label: "Awaiting input", colour: "#f59e0b" };
    case "completed":
      return { label: "Completed", colour: "#22c55e" };
    case "failed":
      return { label: "Failed", colour: "#ef4444" };
    case "cancelled":
      return { label: "Cancelled", colour: "#f59e0b" };
    default:
      return { label: "Idle", colour: "#9898b0" };
  }
}

/** Chat-style running view with user prompt bubble and streaming agent output. */
export function ChatView({
  run,
  logs,
  artifacts,
  onCancel,
  onBackToHome,
  onOpenSettings,
}: ChatViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new content arrives
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [logs.length, Object.keys(artifacts).length]);

  const { label: statusLabel, colour: statusColour } = statusInfo(run.status);

  /** Combine artifacts into a readable output block. */
  const artifactEntries = Object.entries(artifacts);

  return (
    <div className="flex h-full flex-col bg-[#0f0f14] relative">
      {/* Settings gear — top-right */}
      <button
        onClick={onOpenSettings}
        className="absolute top-4 right-4 z-10 rounded p-2 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
        title="Settings"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>

      {/* Scrollable chat area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <div className="mx-auto max-w-2xl flex flex-col gap-4">
          {/* User prompt — right-aligned bubble */}
          <div className="flex justify-end">
            <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
              {run.prompt}
            </div>
          </div>

          {/* Agent output — left-aligned */}
          <div className="flex justify-start">
            <div className="w-full rounded-2xl rounded-bl-md border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
              {/* Status indicator */}
              <div className="flex items-center gap-2 mb-3">
                {isActive(run.status) && (
                  <svg className="animate-spin h-3.5 w-3.5" style={{ color: statusColour }} xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                  </svg>
                )}
                {isTerminal(run.status) && (
                  <div className="h-2.5 w-2.5 rounded-full" style={{ backgroundColor: statusColour }} />
                )}
                <span className="text-xs font-medium" style={{ color: statusColour }}>
                  {statusLabel}
                </span>
                {run.currentStage && isActive(run.status) && (
                  <span className="text-xs text-[#9898b0]">
                    — {run.currentStage}
                  </span>
                )}
              </div>

              {/* Streaming logs */}
              {logs.length > 0 && (
                <div className="font-mono text-xs text-[#e4e4ed] max-h-80 overflow-y-auto mb-3">
                  {logs.map((line, idx) => (
                    <div key={idx} className="whitespace-pre-wrap break-all leading-5">
                      {line}
                    </div>
                  ))}
                </div>
              )}

              {/* Artifacts */}
              {artifactEntries.length > 0 && (
                <div className="flex flex-col gap-2 border-t border-[#2e2e48] pt-3">
                  {artifactEntries.map(([kind, content]) => (
                    <details key={kind} className="group">
                      <summary className="cursor-pointer text-xs font-medium text-[#9898b0] hover:text-[#e4e4ed] transition-colors">
                        {kind}
                      </summary>
                      <pre className="mt-2 font-mono text-xs text-[#e4e4ed] whitespace-pre-wrap break-words max-h-60 overflow-y-auto rounded bg-[#0f0f14] p-3">
                        {content}
                      </pre>
                    </details>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Bottom input bar — always visible */}
      <div className="flex w-full max-w-2xl mx-auto flex-col items-center gap-3 px-6 pb-6 pt-2">
        <div className="flex w-full items-end gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
          <span className="flex-1 text-sm text-[#9898b0] text-center">
            {isActive(run.status) ? "Pipeline is running..." : "Pipeline finished."}
          </span>

          {isActive(run.status) && (
            <button
              onClick={onCancel}
              className="shrink-0 rounded-lg bg-[#ef4444] p-2 text-white hover:bg-red-400 transition-colors"
              title="Cancel pipeline"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}

          {isTerminal(run.status) && (
            <button
              onClick={onBackToHome}
              className="shrink-0 rounded-lg bg-[#6366f1] p-2 text-white hover:bg-[#818cf8] transition-colors"
              title="New run"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="5" x2="12" y2="19" />
                <line x1="5" y1="12" x2="19" y2="12" />
              </svg>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
