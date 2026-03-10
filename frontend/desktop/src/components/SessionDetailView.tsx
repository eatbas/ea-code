import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import type { SessionDetail, RunOptions, CliHealth, AppSettings } from "../types";
import { RunCard } from "./RunCard";
import { PromptInputBar } from "./shared/PromptInputBar";
import { RecentTerminalPanel } from "./shared/RecentTerminalPanel";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";
import { formatDuration, parseUtcTimestamp } from "../utils/formatters";

interface SessionDetailViewProps {
  sessionDetail: SessionDetail | null;
  loading: boolean;
  stageLogs: Record<string, string[]>;
  activeRunId?: string;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup: () => void;
  onRun: (options: RunOptions) => void;
  onPauseRun?: (runId: string) => void;
  onResumeRun?: (runId: string) => void;
  onCancelRun?: (runId: string) => void;
  onBackToHome: () => void;
}

/** Displays a session's run history and allows continuing the conversation. */
export function SessionDetailView({
  sessionDetail,
  loading,
  stageLogs,
  activeRunId,
  cliHealth,
  settings,
  onMissingAgentSetup,
  onRun,
  onPauseRun,
  onResumeRun,
  onCancelRun,
  onBackToHome,
}: SessionDetailViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);
  const recentTerminalRef = useRef<HTMLPreElement>(null);
  const [, setElapsedTick] = useState(0);

  // Scroll to bottom only when switching session or when a new run is added.
  useEffect(() => {
    const el = scrollRef.current;
    if (el) { el.scrollTop = el.scrollHeight; }
  }, [sessionDetail?.id, sessionDetail?.runs.length]);
  const runs = sessionDetail?.runs ?? [];
  const liveRun = [...runs].reverse().find(
    (run) => run.status === "running" || run.status === "waiting_for_input" || run.status === "paused",
  );
  const liveStatusLabel =
    liveRun?.status === "paused"
      ? "Paused"
      : liveRun?.status === "waiting_for_input"
        ? "Awaiting input"
        : "Running";
  const liveStatusColour = liveRun?.status === "paused" ? "#60a5fa" : liveRun?.status === "waiting_for_input" ? "#f59e0b" : "#22c55e";
  const showPause = liveRun?.status === "running" || liveRun?.status === "waiting_for_input";
  const showResume = liveRun?.status === "paused";
  const hasLiveTerminal = !!liveRun && liveRun.id === activeRunId;
  const liveRunStages = liveRun ? liveRun.iterations.flatMap((iter) => iter.stages) : [];
  const recentTerminalStage = liveRun?.currentStage
    ?? [...liveRunStages]
      .reverse()
      .map((stage) => stage.stage)
      .find((stage) => (stageLogs[stage]?.length ?? 0) > 0);
  const recentTerminalLines = hasLiveTerminal && recentTerminalStage
    ? (stageLogs[recentTerminalStage] ?? []).slice(-160)
    : [];
  const recentTerminalLabel = recentTerminalStage?.replace(/_/g, " ");
  const iterationText = liveRun
    ? `Iteration ${Math.max(1, liveRun.currentIteration)}/${Math.max(1, liveRun.maxIterations)}`
    : "";
  const startedAtMs = liveRun ? parseUtcTimestamp(liveRun.startedAt).getTime() : NaN;
  const endedAtMs = liveRun?.completedAt ? parseUtcTimestamp(liveRun.completedAt).getTime() : Date.now();
  const elapsedText = Number.isFinite(startedAtMs) ? formatDuration(Math.max(0, endedAtMs - startedAtMs)) : "0ms";

  useEffect(() => {
    if (!liveRun || (liveRun.status !== "running" && liveRun.status !== "paused")) return;
    const interval = window.setInterval(() => setElapsedTick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [liveRun?.id, liveRun?.status]);

  useEffect(() => {
    const el = recentTerminalRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [recentTerminalStage, recentTerminalLines.length]);

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
    <div className="flex h-full min-h-0 flex-col bg-[#0f0f14]">
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
      <div ref={scrollRef} className="app-scrollbar min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-6 [scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {sessionDetail.runs.length === 0 && (
            <div className="text-center text-sm text-[#9898b0] py-8">
              No runs in this session yet. Send a prompt to get started.
            </div>
          )}

          {sessionDetail.runs.map((run) => (
            <RunCard key={run.id} run={run} settings={settings} />
          ))}
        </div>
      </div>

      {/* Bottom input bar */}
      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        {liveRun && (
          <RecentTerminalPanel
            label={recentTerminalLabel}
            lines={recentTerminalLines}
            terminalRef={recentTerminalRef}
          />
        )}
        {liveRun ? (
          <div className="flex w-full items-center gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
            <div className="flex items-center gap-2 flex-1">
              {showResume ? (
                <div className="h-3.5 w-3.5 rounded-full border-2 border-[#3b82f6]" />
              ) : (
                <svg className="animate-spin h-3.5 w-3.5" style={{ color: liveStatusColour }} xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                </svg>
              )}
              <span className="text-sm text-[#9898b0]">{liveStatusLabel}... | {iterationText} | {elapsedText}</span>
            </div>
            {showPause && onPauseRun && (
              <button
                onClick={() => onPauseRun(liveRun.id)}
                className="shrink-0 rounded-lg bg-[#2563eb] p-2 text-white hover:bg-[#3b82f6] transition-colors"
                title="Pause pipeline"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="5" width="4" height="14" rx="1" />
                  <rect x="14" y="5" width="4" height="14" rx="1" />
                </svg>
              </button>
            )}
            {showResume && onResumeRun && (
              <button
                onClick={() => onResumeRun(liveRun.id)}
                className="shrink-0 rounded-lg bg-[#22c55e] p-2 text-white hover:bg-[#16a34a] transition-colors"
                title="Resume pipeline"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </button>
            )}
            {onCancelRun && (
              <button
                onClick={() => onCancelRun(liveRun.id)}
                className="shrink-0 rounded-lg bg-[#ef4444] p-2 text-white hover:bg-red-400 transition-colors"
                title="Cancel pipeline"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            )}
          </div>
        ) : (
          <PromptInputBar
            placeholder="Continue this session..."
            cliHealth={cliHealth}
            settings={settings}
            onMissingAgentSetup={onMissingAgentSetup}
            onSubmit={onRun}
          />
        )}

        {/* Workspace path + Open in VS Code */}
        <WorkspaceFooter path={sessionDetail.projectPath} />
      </div>
    </div>
  );
}
