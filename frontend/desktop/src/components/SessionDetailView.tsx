import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import type { SessionDetail, RunOptions, CliHealth, AppSettings } from "../types";
import { useElapsedTimer } from "../hooks/useElapsedTimer";
import { useRecentTerminal } from "../hooks/useRecentTerminal";
import { isActiveStatusValue, isLiveSessionStatus, statusToneClasses } from "../utils/statusHelpers";
import { RunCard } from "./RunCard";
import { PromptInputBar } from "./shared/PromptInputBar";
import { RecentTerminalPanel } from "./shared/RecentTerminalPanel";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";
import { PipelineControlBar } from "./shared/PipelineControlBar";

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
  onLoadMore?: () => void;
  loadingMore?: boolean;
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
  onLoadMore,
  loadingMore,
  onBackToHome,
}: SessionDetailViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) { el.scrollTop = el.scrollHeight; }
  }, [sessionDetail?.id, sessionDetail?.runs.length]);

  const runs = sessionDetail?.runs ?? [];
  const liveRun = [...runs].reverse().find((run) => isLiveSessionStatus(run.status));

  const liveStatusLabel =
    liveRun?.status === "paused"
      ? "Paused"
      : liveRun?.status === "waiting_for_input"
        ? "Awaiting input"
        : "Running";
  const liveStatusClasses = statusToneClasses(liveRun?.status);
  const showPause = isActiveStatusValue(liveRun?.status);
  const showResume = liveRun?.status === "paused";
  const hasLiveTerminal = !!liveRun && liveRun.id === activeRunId;

  const liveRunStages = liveRun ? liveRun.iterations.flatMap((iter) => iter.stages) : [];
  const terminal = useRecentTerminal(
    hasLiveTerminal ? stageLogs : {},
    liveRun?.currentStage,
    liveRunStages,
  );

  const iterationText = liveRun
    ? `Iteration ${Math.max(1, liveRun.currentIteration)}/${Math.max(1, liveRun.maxIterations)}`
    : "";
  const elapsedText = useElapsedTimer(liveRun?.status, liveRun?.startedAt, liveRun?.completedAt);

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
          {sessionDetail.totalRuns} {sessionDetail.totalRuns === 1 ? "run" : "runs"}
        </span>
      </div>

      <div ref={scrollRef} className="app-scrollbar min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-6 [scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {sessionDetail.runs.length < sessionDetail.totalRuns && onLoadMore && (
            <div className="flex justify-center">
              <button
                onClick={onLoadMore}
                disabled={loadingMore}
                className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] px-4 py-2 text-xs text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors disabled:opacity-50"
              >
                {loadingMore ? "Loading..." : `Load earlier runs (${sessionDetail.totalRuns - sessionDetail.runs.length} more)`}
              </button>
            </div>
          )}

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

      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        {liveRun && (
          <RecentTerminalPanel
            label={terminal.label}
            lines={terminal.lines}
            terminalRef={terminal.terminalRef}
          />
        )}
        {liveRun ? (
          <PipelineControlBar
            statusLabel={liveStatusLabel}
            statusClassName={liveStatusClasses.text}
            iterationText={iterationText}
            elapsedText={elapsedText}
            isPaused={!!showResume}
            showPause={!!showPause}
            showResume={!!showResume}
            onPause={onPauseRun ? () => onPauseRun(liveRun.id) : undefined}
            onResume={onResumeRun ? () => onResumeRun(liveRun.id) : undefined}
            onCancel={() => onCancelRun?.(liveRun.id)}
          />
        ) : (
          <PromptInputBar
            placeholder="Continue this session..."
            cliHealth={cliHealth}
            settings={settings}
            onMissingAgentSetup={onMissingAgentSetup}
            onSubmit={onRun}
          />
        )}
        <WorkspaceFooter path={sessionDetail.projectPath} />
      </div>
    </div>
  );
}
