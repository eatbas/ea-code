import type { ReactNode } from "react";
import { useCallback, useState } from "react";
import { ArrowDown, ArrowUp, ChevronDown, Clipboard } from "lucide-react";
import type { ConversationDetail } from "../../types";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";
import type { UsePipelineSessionReturn } from "../../hooks/usePipelineSession";
import { useStickyAutoScroll } from "../../hooks/useStickyAutoScroll";
import { PipelineConversationView } from "./PipelineConversationView";
import { ConversationEmptyState } from "./ConversationEmptyState";
import { ConversationTranscript } from "./ConversationTranscript";

interface ConversationMainProps {
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  pipeline: UsePipelineSessionReturn;
  pipelinePrompt: string;
  planReviewPhase: PlanReviewPhase;
  onResume: () => Promise<void>;
  onRedoReview: () => Promise<void>;
  onStop: () => Promise<void>;
  onRetryStage: (stageIndex: number) => Promise<void>;
  retryingStageIndex: number | null;
}

export function ConversationMain({
  activeConversation,
  activeDraft,
  pipeline,
  pipelinePrompt,
  planReviewPhase,
  onResume,
  onRedoReview,
  onStop,
  onRetryStage,
  retryingStageIndex,
}: ConversationMainProps): ReactNode {
  const messages = activeConversation?.messages ?? [];
  const lastMessageLength = messages[messages.length - 1]?.content.length ?? 0;
  const scrollKey = `${messages.length}:${lastMessageLength}:${activeDraft.length}`;
  const { scrollRef, showScrollButtons, scrollToBottom, scrollToTop } = useStickyAutoScroll<HTMLDivElement>(scrollKey);

  // All hooks must be called before any conditional return (React Rules of Hooks).
  const [debugOpen, setDebugOpen] = useState(false);
  const [copiedDebug, setCopiedDebug] = useState(false);
  const debugLog = pipeline.debugLog;

  const handleCopyDebugLog = useCallback(async () => {
    if (!debugLog.trim()) return;
    await navigator.clipboard.writeText(debugLog);
    setCopiedDebug(true);
    setTimeout(() => setCopiedDebug(false), 2000);
  }, [debugLog]);

  if (pipeline.stages.length > 0 || pipeline.running || pipeline.userPrompt) {
    return (
      <PipelineConversationView
        userPrompt={pipelinePrompt || pipeline.userPrompt}
        stages={pipeline.stages}
        debugLog={pipeline.debugLog}
        running={pipeline.running}
        currentStageName={pipeline.currentStageName}
        pipelineStartedAt={pipeline.pipelineStartedAt}
        onResume={onResume}
        onRedoReview={onRedoReview}
        onStop={onStop}
        onRetryStage={onRetryStage}
        retryingStageIndex={retryingStageIndex}
        planReviewPhase={planReviewPhase}
        activeConversation={activeConversation}
        activeDraft={activeDraft}
      />
    );
  }

  const showDebugPanel = import.meta.env.VITE_MAESTRO_DEV === "true"
    && activeConversation
    && debugLog.trim().length > 0;

  return (
    <div className="relative min-h-0 flex-1">
      <div ref={scrollRef} className="h-full overflow-y-auto px-5 py-5">
        {activeConversation ? (
          <>
            <ConversationTranscript activeConversation={activeConversation} activeDraft={activeDraft} />
            {showDebugPanel && (
              <div className="mx-auto mt-6 max-w-3xl rounded-2xl border border-edge bg-panel">
                <div className="flex items-center justify-between border-b border-edge px-4 py-3">
                  <button
                    type="button"
                    onClick={() => setDebugOpen((o) => !o)}
                    className="flex min-w-0 flex-1 items-center gap-3 text-left"
                    aria-expanded={debugOpen}
                  >
                    <div className="min-w-0 flex-1">
                      <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                        Task Debug
                      </p>
                      <p className="text-xs text-fg-muted">
                        Backend trace. Copy and send for diagnosis.
                      </p>
                    </div>
                    <ChevronDown
                      size={14}
                      className={`shrink-0 text-fg-muted transition-transform ${debugOpen ? "rotate-180" : ""}`}
                    />
                  </button>
                  <button
                    type="button"
                    onClick={() => { void handleCopyDebugLog(); }}
                    disabled={!debugLog.trim()}
                    className="ml-3 inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-3 py-1.5 text-xs font-semibold text-fg transition-colors hover:bg-active disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <Clipboard size={12} />
                    {copiedDebug ? "Copied" : "Copy Log"}
                  </button>
                </div>
                {debugOpen && (
                  <div className="max-h-56 overflow-auto px-4 py-3 pipeline-scroll">
                    <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-fg-muted">
                      {debugLog}
                    </pre>
                  </div>
                )}
              </div>
            )}
          </>
        ) : (
          <ConversationEmptyState />
        )}
      </div>

      {showScrollButtons && (
        <div className="absolute bottom-3 right-6 flex flex-col gap-1.5">
          <button
            type="button"
            onClick={scrollToTop}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-running-dot text-send-text shadow-lg transition-colors hover:bg-send-bg-hover"
            title="Scroll to top"
          >
            <ArrowUp size={14} strokeWidth={2.2} />
          </button>
          <button
            type="button"
            onClick={scrollToBottom}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-running-dot text-send-text shadow-lg transition-colors hover:bg-send-bg-hover"
            title="Scroll to bottom"
          >
            <ArrowDown size={14} strokeWidth={2.2} />
          </button>
        </div>
      )}
    </div>
  );
}
