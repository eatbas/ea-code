import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { SessionDetail, RunOptions, CliHealth, AppSettings } from "../types";
import { useToast } from "./shared/Toast";
import { RunCard } from "./RunCard";
import { PromptInputBar } from "./shared/PromptInputBar";

interface SessionDetailViewProps {
  sessionDetail: SessionDetail | null;
  loading: boolean;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup: () => void;
  onRun: (options: RunOptions) => void;
  onBackToHome: () => void;
}

/** Displays a session's run history and allows continuing the conversation. */
export function SessionDetailView({
  sessionDetail,
  loading,
  cliHealth,
  settings,
  onMissingAgentSetup,
  onRun,
  onBackToHome,
}: SessionDetailViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);
  const toast = useToast();

  // Scroll to bottom when session detail loads
  useEffect(() => {
    const el = scrollRef.current;
    if (el) { el.scrollTop = el.scrollHeight; }
  }, [sessionDetail]);

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Loading session...</span>
      </div>
    );
  }

  if (!sessionDetail) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Session not found.</span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      {/* Header */}
      <div className="flex items-center gap-3 border-b border-[#2e2e48] px-6 py-3">
        <button
          onClick={onBackToHome}
          className="rounded p-1 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="Back to home"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
        </button>
        <h2 className="text-sm font-medium text-[#e4e4ed] truncate">
          {sessionDetail.title || "Session"}
        </h2>
        <span className="text-xs text-[#6f7086]">
          {sessionDetail.runs.length} {sessionDetail.runs.length === 1 ? "run" : "runs"}
        </span>
      </div>

      {/* Scrollable run history */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {sessionDetail.runs.length === 0 && (
            <div className="text-center text-sm text-[#9898b0] py-8">
              No runs in this session yet. Send a prompt to get started.
            </div>
          )}

          {sessionDetail.runs.map((run) => (
            <RunCard key={run.id} run={run} />
          ))}
        </div>
      </div>

      {/* Bottom input bar */}
      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        <PromptInputBar
          placeholder="Continue this session..."
          cliHealth={cliHealth}
          settings={settings}
          onMissingAgentSetup={onMissingAgentSetup}
          onSubmit={onRun}
        />

        {/* Workspace path + Open in VS Code */}
        <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
          <span className="truncate" title={sessionDetail.projectPath}>
            {sessionDetail.projectPath}
          </span>
          <button
            onClick={() => {
              void invoke("open_in_vscode", { path: sessionDetail.projectPath }).catch(() => {
                toast.error("Failed to open VS Code.");
              });
            }}
            className="ml-4 flex shrink-0 items-center gap-1.5 rounded px-2 py-0.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
            title="Open in VS Code"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M16 3l5 3v12l-5 3L2 12l5-3" />
              <path d="M16 3L7 12l9 9" />
              <path d="M16 3v18" />
            </svg>
            Open in VS Code
          </button>
        </div>
      </div>
    </div>
  );
}
