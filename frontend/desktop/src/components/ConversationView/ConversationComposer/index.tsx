import type { ReactNode } from "react";
import { useCallback, useRef } from "react";
import type { PlanReviewPhase } from "../../../hooks/usePlanReview";
import type { AgentSelection, ProviderInfo } from "../../../types";
import { useAutoResizeTextarea } from "../../../hooks/useAutoResizeTextarea";
import { type PendingImage, useImageAttachments } from "../../../hooks/useImageAttachments";
import { ComposerToolbar } from "./ComposerToolbar";
import { ImageThumbnails } from "./ImageThumbnails";
import { PromptInput } from "./PromptInput";
import { usePromptHistoryNavigation } from "./usePromptHistoryNavigation";

export type PipelineMode = "auto" | "simple" | "code";

interface ConversationComposerProps {
  providers: ProviderInfo[];
  agent: AgentSelection | null;
  prompt: string;
  promptHistory: string[];
  locked: boolean;
  sending: boolean;
  stopping: boolean;
  activeRunning: boolean;
  pipelineRunning: boolean;
  pipelineMode: PipelineMode;
  pipelineDone: boolean;
  sidecarReady: boolean | null;
  thinkingLevel: string;
  thinkingOptions: { value: string; label: string }[] | undefined;
  workspacePath: string;
  conversationId: string | null;
  onPipelineModeChange: (mode: PipelineMode) => void;
  onAgentChange: (agent: AgentSelection) => void;
  onThinkingChange: (value: string) => void;
  onPromptChange: (prompt: string) => void;
  onSend: (prompt: string, pendingImages?: PendingImage[]) => Promise<void>;
  onStop: () => Promise<void>;
  onResumePipeline?: () => void;
  onNewPipeline?: () => void;
  planReviewPhase?: PlanReviewPhase;
}

export function ConversationComposer({
  providers,
  agent,
  prompt,
  promptHistory,
  locked,
  sending,
  stopping,
  activeRunning,
  pipelineRunning,
  pipelineMode,
  pipelineDone,
  sidecarReady,
  thinkingLevel,
  thinkingOptions,
  workspacePath,
  conversationId,
  onPipelineModeChange,
  onAgentChange,
  onThinkingChange,
  onPromptChange,
  onSend,
  onStop,
  onResumePipeline,
  onNewPipeline,
  planReviewPhase,
}: ConversationComposerProps): ReactNode {
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const resetHistoryRef = useRef<() => void>(() => undefined);
  useAutoResizeTextarea(textareaRef, prompt);

  const {
    addImages,
    buildPromptWithImages,
    clearImages,
    pendingImages,
    hasImages,
    allPreviews,
    removeImage,
  } = useImageAttachments(workspacePath, conversationId);

  const isReviewing = planReviewPhase === "reviewing";
  const isEditing = planReviewPhase === "editing";
  const isSubmittingEdit = planReviewPhase === "submitting_edit";
  const inReviewFlow = isReviewing || isEditing || isSubmittingEdit;
  const composerDisabled = pipelineRunning || inReviewFlow;
  const submitDisabled = stopping
    || sending
    || activeRunning
    || (pipelineMode === "simple" && !agent)
    || prompt.trim().length === 0
    || sidecarReady !== true;

  const handleImagePaste = useCallback((files: File[]) => {
    void addImages(files);
  }, [addImages]);

  const handleSubmit = useCallback(async () => {
    const trimmed = prompt.trim();
    if (!trimmed || !agent) {
      return;
    }

    const finalPrompt = buildPromptWithImages(trimmed);
    await onSend(finalPrompt, pendingImages);
    onPromptChange("");
    clearImages();
    resetHistoryRef.current();
  }, [agent, buildPromptWithImages, clearImages, onPromptChange, onSend, pendingImages, prompt]);

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
      <div className="rounded-[20px] border border-edge bg-panel shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        {hasImages && (
          <ImageThumbnails
            previews={allPreviews}
            onRemove={removeImage}
          />
        )}
        <PromptInput
          prompt={prompt}
          disabled={composerDisabled}
          placeholder={pipelineRunning ? "Pipeline is running..." : "Describe the task you want the agent to handle."}
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
            sending={sending}
            stopping={stopping}
            activeRunning={activeRunning}
            pipelineRunning={pipelineRunning}
            pipelineMode={pipelineMode}
            pipelineDone={pipelineDone}
            sidecarReady={sidecarReady}
            thinkingLevel={thinkingLevel}
            thinkingOptions={thinkingOptions}
            onPipelineModeChange={onPipelineModeChange}
            onAgentChange={onAgentChange}
            onThinkingChange={onThinkingChange}
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
