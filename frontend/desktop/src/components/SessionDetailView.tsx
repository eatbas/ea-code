import type { ReactNode } from "react";
import { useMemo } from "react";
import type { SessionDetail, RunOptions, RunSummary, ProviderInfo, AppSettings } from "../types";
import { useElapsedTimer } from "../hooks/useElapsedTimer";
import { useRecentTerminal } from "../hooks/useRecentTerminal";
import { useStickyAutoScroll } from "../hooks/useStickyAutoScroll";
import { isActiveStatusValue, isLiveSessionStatus, statusToneClasses } from "../utils/statusHelpers";
import { RunCard } from "./RunCard";
import { AssistantMessageBubble } from "./shared/AssistantMessageBubble";
import { PromptInputBar } from "./shared/PromptInputBar";
import { RecentTerminalPanel } from "./shared/RecentTerminalPanel";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";
import { PipelineControlBar } from "./shared/PipelineControlBar";

interface SessionDetailViewProps {
  sessionDetail: SessionDetail | null;
  loading: boolean;
  stageLogs: Record<string, string[]>;
  activeRunId?: string;
  providers: ProviderInfo[];
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

/** Displays a session's conversation history and allows continuing the conversation. */
export function SessionDetailView({
  sessionDetail,
  loading,
  stageLogs,
  activeRunId,
  providers,
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
  const messages = sessionDetail?.messages ?? [];
  const runs = sessionDetail?.runs ?? [];
  const hasMessages = messages.length > 0;
  const historyDependencyKey = `${sessionDetail?.id ?? "none"}:${runs.length}:${messages.length}`;
  const { scrollRef, onScroll } = useStickyAutoScroll<HTMLDivElement>(historyDependencyKey);

  // Build run lookup by ID for O(1) access from message runId
  const runById = useMemo(() => {
    const map = new Map<string, RunSummary>();
    for (const run of runs) {
      map.set(run.id, run);
    }
    return map;
  }, [runs]);

  const assistantLinkedRunIds = useMemo(() => {
    return new Set(
      messages
        .filter((message) => message.role === "assistant" && message.runId)
        .map((message) => message.runId as string),
    );
  }, [messages]);

  const referencedRunIds = useMemo(() => {
    return new Set(
      messages
        .filter((message) => message.runId)
        .map((message) => message.runId as string),
    );
  }, [messages]);

  // Runs that aren't linked from any assistant message (orphan runs)
  const orphanRuns = useMemo(() => {
    if (!hasMessages) return [];
    return runs.filter((run) => !referencedRunIds.has(run.id));
  }, [runs, referencedRunIds, hasMessages]);

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

  const terminal = useRecentTerminal(
    hasLiveTerminal ? stageLogs : {},
    liveRun?.currentStage ?? undefined,
    [],
  );

  const iterationText = liveRun
    ? `Iteration ${Math.max(1, liveRun.currentIteration ?? 1)}/${Math.max(1, liveRun.maxIterations)}`
    : "";
  const elapsedText = useElapsedTimer(liveRun?.status, liveRun?.startedAt, liveRun?.completedAt ?? undefined);

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

      <div ref={scrollRef} onScroll={onScroll} className="app-scrollbar min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-6 [scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {runs.length < sessionDetail.totalRuns && onLoadMore && (
            <div className="flex justify-center">
              <button
                onClick={onLoadMore}
                disabled={loadingMore}
                className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] px-4 py-2 text-xs text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors disabled:opacity-50"
              >
                {loadingMore ? "Loading..." : `Load earlier runs (${sessionDetail.totalRuns - runs.length} more)`}
              </button>
            </div>
          )}

          {runs.length === 0 && messages.length === 0 && (
            <div className="text-center text-sm text-[#9898b0] py-8">
              No messages yet. Send a prompt to get started.
            </div>
          )}

          {hasMessages ? (
            <>
              <MessageTimeline
                messages={messages}
                runById={runById}
                assistantLinkedRunIds={assistantLinkedRunIds}
                settings={settings}
              />
              {/* Render runs not linked from any assistant message */}
              {orphanRuns.map((run) => (
                <RunCard key={run.id} run={run} settings={settings} />
              ))}
            </>
          ) : (
            /* Fallback: legacy sessions without messages.jsonl — render RunCards directly */
            runs.map((run) => (
              <RunCard key={run.id} run={run} settings={settings} />
            ))
          )}
        </div>
      </div>

      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        {liveRun && (
          <RecentTerminalPanel
            label={terminal.label}
            lines={terminal.lines}
            terminalRef={terminal.terminalRef}
            onTerminalScroll={terminal.onTerminalScroll}
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
            providers={providers}
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

// ---------------------------------------------------------------------------
// Message timeline sub-component
// ---------------------------------------------------------------------------

interface MessageTimelineProps {
  messages: SessionDetail["messages"];
  runById: Map<string, RunSummary>;
  assistantLinkedRunIds: Set<string>;
  settings: AppSettings | null;
}

/** Renders the conversation as a chat timeline driven by messages.jsonl. */
function MessageTimeline({
  messages,
  runById,
  assistantLinkedRunIds,
  settings,
}: MessageTimelineProps): ReactNode {
  return (
    <>
      {messages.map((msg, idx) => {
        if (msg.role === "user") {
          const linkedRun = msg.runId ? runById.get(msg.runId) : undefined;
          const shouldRenderRunCard = !!linkedRun && !assistantLinkedRunIds.has(linkedRun.id);

          return (
            <div key={`msg-${idx}`} className="flex flex-col gap-3">
              <div className="flex justify-end">
                <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
                  {msg.content}
                </div>
              </div>
              {shouldRenderRunCard && (
                <RunCard
                  run={linkedRun}
                  settings={settings}
                  hidePromptBubble
                />
              )}
            </div>
          );
        }

        // Assistant message — show bubble + linked RunCard (if available)
        const linkedRun = msg.runId ? runById.get(msg.runId) : undefined;
        return (
          <div key={`msg-${idx}`} className="flex flex-col gap-3">
            <AssistantMessageBubble
              content={msg.content}
              timestamp={msg.timestamp}
            />
            {linkedRun && (
              <RunCard
                run={linkedRun}
                settings={settings}
                hidePromptBubble
              />
            )}
          </div>
        );
      })}
    </>
  );
}
