import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import type { SessionDetail, RunOptions, CliHealth } from "../types";
import { RunCard } from "./RunCard";
import { PromptInputBar } from "./shared/PromptInputBar";

interface SessionDetailViewProps {
  sessionDetail: SessionDetail | null;
  loading: boolean;
  cliHealth: CliHealth | null;
  onRun: (options: RunOptions) => void;
  onBackToHome: () => void;
}

/** Displays a session's run history and allows continuing the conversation. */
export function SessionDetailView({
  sessionDetail,
  loading,
  cliHealth,
  onRun,
  onBackToHome,
}: SessionDetailViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);

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
          onSubmit={onRun}
        />
      </div>
    </div>
  );
}
