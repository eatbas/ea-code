import type { ReactNode } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Square } from "lucide-react";
import type { PlanReviewPhase } from "../../../hooks/usePlanReview";
import type { AgentSelection, ProviderInfo } from "../../../types";
import { useAutoResizeTextarea } from "../../../hooks/useAutoResizeTextarea";
import { type PendingImage, useImageAttachments } from "../../../hooks/useImageAttachments";
import type { SymphonyStartupStatus } from "../../../utils/symphonyStartup";
import { ImagePreviewModal } from "../../shared/ImagePreviewModal";
import { ComposerToolbar } from "./ComposerToolbar";
import { ImageThumbnails } from "./ImageThumbnails";
import { PromptInput } from "./PromptInput";
import { QueuedPromptBanner } from "./QueuedPromptBanner";
import { StartupBanner } from "./StartupBanner";
import { usePromptHistoryNavigation } from "./usePromptHistoryNavigation";

export type PipelineMode = "auto" | "simple" | "code";

function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${String(minutes)}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${String(seconds)}s`;
}

/** Status bar shown while the agent is running in Simple Task mode.
 *  Layout mirrors PipelineStatusBar: green light, pulsing label, stop, timer. */
function SimpleTaskStatusBar({
  startedAt,
  now,
  stopping,
  onStop,
}: {
  startedAt: number;
  now: number;
  stopping: boolean;
  onStop: () => void;
}): ReactNode {
  const elapsed = formatElapsed(now - startedAt);
  return (
    <div className="relative overflow-hidden bg-surface rounded-t-[20px]">
      {/* Green flowing light */}
      <div className="absolute inset-x-0 top-0 h-[2px]">
        <div className="h-full w-1/3 animate-[flowRight_2s_ease-in-out_infinite] rounded-full bg-gradient-to-r from-transparent via-running-dot to-transparent" />
      </div>
      <div className="flex items-center justify-between px-5 py-2">
        <div className="flex items-center gap-1.5">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-running-dot" />
          <span className="text-xs font-semibold animate-pulse text-running-dot">
            Thinking
          </span>
        </div>
        <button
          type="button"
          onClick={onStop}
          disabled={stopping}
          className="inline-flex items-center gap-2 rounded-lg border border-error-border bg-error-bg px-4 py-1.5 text-xs font-semibold text-error-text transition-colors hover:opacity-80 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Square size={10} fill="currentColor" />
          {stopping ? "Stopping..." : "Stop"}
        </button>
        <span className="text-xs font-mono text-fg-faint">
          {elapsed}
        </span>
      </div>
    </div>
  );
}

interface ConversationComposerProps {
  providers: ProviderInfo[];
  agent: AgentSelection | null;
  startupStatus: SymphonyStartupStatus;
  prompt: string;
  promptHistory: string[];
  locked: boolean;
  modelChangeable: boolean;
  sending: boolean;
  stopping: boolean;
  activeRunning: boolean;
  pipelineRunning: boolean;
  pipelineMode: PipelineMode;
  pipelineDone: boolean;
  thinkingLevel: string;
  thinkingOptions: { value: string; label: string }[] | undefined;
  workspacePath: string;
  conversationId: string | null;
  onPipelineModeChange: (mode: PipelineMode) => void;
  onAgentChange: (agent: AgentSelection) => void;
  onThinkingChange: (value: string) => void;
  isKimi: boolean;
  isResume: boolean;
  kimiSwarmEnabled: boolean;
  kimiRalphIterations: number;
  redoSwarm: boolean;
  onRedoSwarmChange: (value: boolean) => void;
  onSwarmChange: (value: string) => void;
  onRalphIterationsChange: (value: string) => void;
  onPromptChange: (prompt: string) => void;
  onSend: (prompt: string, pendingImages?: PendingImage[]) => Promise<void>;
  onStop: () => Promise<void>;
  onResumePipeline?: () => void;
  onNewPipeline?: () => void;
  planReviewPhase?: PlanReviewPhase;
  onOpenCliSetup: () => void;
}

export function ConversationComposer({
  providers,
  agent,
  startupStatus,
  prompt,
  promptHistory,
  locked,
  modelChangeable,
  sending,
  stopping,
  activeRunning,
  pipelineRunning,
  pipelineMode,
  pipelineDone,
  thinkingLevel,
  thinkingOptions,
  workspacePath,
  conversationId,
  onPipelineModeChange,
  onAgentChange,
  onThinkingChange,
  isKimi,
  isResume,
  kimiSwarmEnabled,
  kimiRalphIterations,
  redoSwarm,
  onRedoSwarmChange,
  onSwarmChange,
  onRalphIterationsChange,
  onPromptChange,
  onSend,
  onStop,
  onResumePipeline,
  onNewPipeline,
  planReviewPhase,
  onOpenCliSetup,
}: ConversationComposerProps): ReactNode {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const resetHistoryRef = useRef<() => void>(() => undefined);
  useAutoResizeTextarea(textareaRef, prompt);

  const [queuedPrompt, setQueuedPrompt] = useState<string | null>(null);
  const [runStartedAt, setRunStartedAt] = useState<number>(0);
  const [now, setNow] = useState(Date.now());
  const [previewImageSrc, setPreviewImageSrc] = useState<string | null>(null);
  const prevActiveRunningRef = useRef(activeRunning);

  const {
    addImages,
    buildPromptWithImages,
    clearImages,
    pendingImages,
    hasImages,
    allPreviews,
    removeImage,
  } = useImageAttachments(workspacePath, conversationId);

  // Track when the agent starts running (for the timer).
  useEffect(() => {
    if (activeRunning && !prevActiveRunningRef.current) {
      setRunStartedAt(Date.now());
    }
    prevActiveRunningRef.current = activeRunning;
  }, [activeRunning]);

  // Live clock tick for elapsed timer while agent is running.
  useEffect(() => {
    if (!activeRunning) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [activeRunning]);

  // Auto-send queued prompt when the agent finishes.
  useEffect(() => {
    if (!activeRunning && queuedPrompt !== null && pipelineMode === "simple") {
      const pendingPrompt = queuedPrompt;
      setQueuedPrompt(null);
      void onSend(pendingPrompt);
    }
  }, [activeRunning, queuedPrompt, pipelineMode, onSend]);

  const isReviewing = planReviewPhase === "reviewing";
  const isEditing = planReviewPhase === "editing";
  const isSubmittingEdit = planReviewPhase === "submitting_edit";
  const inReviewFlow = isReviewing || isEditing || isSubmittingEdit;
  const composerDisabled = pipelineRunning || inReviewFlow;
  const isSimpleRunning = pipelineMode === "simple" && activeRunning;
  const submitDisabled = stopping
    || sending
    || (activeRunning && pipelineMode !== "simple")
    || (pipelineMode === "simple" && !agent)
    || prompt.trim().length === 0
    || startupStatus.phase !== "connected";

  const handleImagePaste = useCallback((files: File[]) => {
    void addImages(files);
  }, [addImages]);

  const handleSubmit = useCallback(async () => {
    const trimmed = prompt.trim();
    if (!trimmed) {
      return;
    }

    if (pipelineMode !== "code" && !agent) {
      return;
    }

    // Queue the message if the agent is busy in Simple Task mode.
    if (isSimpleRunning) {
      const finalPrompt = buildPromptWithImages(trimmed);
      setQueuedPrompt(finalPrompt);
      onPromptChange("");
      clearImages();
      resetHistoryRef.current();
      return;
    }

    const finalPrompt = buildPromptWithImages(trimmed);
    await onSend(finalPrompt, pendingImages);
    onPromptChange("");
    clearImages();
    resetHistoryRef.current();
  }, [agent, buildPromptWithImages, clearImages, isSimpleRunning, onPromptChange, onSend, pendingImages, prompt]);

  const promptNavigation = usePromptHistoryNavigation({
    prompt,
    promptHistory,
    textareaRef,
    onPromptChange,
    onSubmit: handleSubmit,
    disabled: submitDisabled,
  });
  resetHistoryRef.current = promptNavigation.resetHistory;

  return (
    <div className="bg-surface px-5 pb-2 pt-1">
      {previewImageSrc !== null && (
        <ImagePreviewModal
          src={previewImageSrc}
          onClose={() => setPreviewImageSrc(null)}
        />
      )}
      <div className="rounded-[20px] border border-edge bg-panel shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        {queuedPrompt !== null && (
          <QueuedPromptBanner
            prompt={queuedPrompt}
            onDelete={() => setQueuedPrompt(null)}
          />
        )}

        {/* Simple task status bar — mirrors PipelineStatusBar exactly */}
        {isSimpleRunning && (
          <SimpleTaskStatusBar
            startedAt={runStartedAt}
            now={now}
            stopping={stopping}
            onStop={() => { void onStop(); }}
          />
        )}

        <StartupBanner
          status={startupStatus}
          onOpenCliSetup={onOpenCliSetup}
        />

        {hasImages && (
          <ImageThumbnails
            previews={allPreviews}
            onRemove={removeImage}
            onPreview={setPreviewImageSrc}
          />
        )}

        <PromptInput
          prompt={prompt}
          disabled={composerDisabled}
          placeholder={
            queuedPrompt !== null
              ? "Message queued — will send when agent finishes..."
              : pipelineRunning
                ? "Pipeline is running..."
                : "Describe the task you want the agent to handle."
          }
          textareaRef={textareaRef}
          onPromptChange={promptNavigation.handlePromptChange}
          onKeyDown={promptNavigation.handlePromptKeyDown}
          onImagePaste={handleImagePaste}
        />

        {!inReviewFlow && (
          <ComposerToolbar
            providers={providers}
            agent={agent}
            locked={locked}
            modelChangeable={modelChangeable}
            sending={sending}
            stopping={stopping}
            activeRunning={activeRunning}
            pipelineRunning={pipelineRunning}
            pipelineMode={pipelineMode}
            pipelineDone={pipelineDone}
            startupPhase={startupStatus.phase}
            thinkingLevel={thinkingLevel}
            thinkingOptions={thinkingOptions}
            onPipelineModeChange={onPipelineModeChange}
            onAgentChange={onAgentChange}
            onThinkingChange={onThinkingChange}
            isKimi={isKimi}
            isResume={isResume}
            kimiSwarmEnabled={kimiSwarmEnabled}
            kimiRalphIterations={kimiRalphIterations}
            redoSwarm={redoSwarm}
            onRedoSwarmChange={onRedoSwarmChange}
            onSwarmChange={onSwarmChange}
            onRalphIterationsChange={onRalphIterationsChange}
            onSubmit={handleSubmit}
            onStop={onStop}
            onResumePipeline={onResumePipeline}
            onNewPipeline={onNewPipeline}
            submitDisabled={submitDisabled}
          />
        )}
      </div>
    </div>
  );
}
