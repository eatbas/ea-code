import type { ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { ArrowRight, Plus, RotateCcw, Square } from "lucide-react";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";
import type { AgentSelection, ProviderInfo } from "../../types";
import { useAutoResizeTextarea } from "../../hooks/useAutoResizeTextarea";
import { PopoverSelect } from "../shared/PopoverSelect";
import {
  modelOptionsFromProvider,
  providerDisplayName,
} from "../shared/constants";

export type PipelineMode = "auto" | "simple" | "code";

const PIPELINE_OPTIONS: { value: PipelineMode; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "simple", label: "Simple Task" },
  { value: "code", label: "Code Pipeline" },
];

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
  /** Whether a previous pipeline run has finished (completed or failed). */
  pipelineDone: boolean;
  /** `null` = still starting, `true` = ready, `false` = failed. */
  sidecarReady: boolean | null;
  onPipelineModeChange: (mode: PipelineMode) => void;
  onAgentChange: (agent: AgentSelection) => void;
  onPromptChange: (prompt: string) => void;
  onSend: (prompt: string) => Promise<void>;
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
  onPipelineModeChange,
  onAgentChange,
  onPromptChange,
  onSend,
  onStop,
  onResumePipeline,
  onNewPipeline,
  planReviewPhase,
}: ConversationComposerProps): ReactNode {
  const [openSelect, setOpenSelect] = useState<"pipeline" | "provider" | "model" | null>(null);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftBeforeHistory, setDraftBeforeHistory] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const availableProviders = useMemo(
    () => providers.filter((provider) => provider.available),
    [providers],
  );
  const selectedProvider = availableProviders.find((provider) => provider.name === agent?.provider)
    ?? availableProviders[0];
  const selectedProviderValue = selectedProvider?.name ?? "";
  const selectedModelValue = selectedProvider?.models.includes(agent?.model ?? "")
    ? agent?.model ?? ""
    : selectedProvider?.models[0] ?? "";
  const modelOptions = modelOptionsFromProvider(selectedProvider);
  const providerOptions = useMemo(
    () => availableProviders.map((provider) => ({
      value: provider.name,
      label: providerDisplayName(provider.name),
    })),
    [availableProviders],
  );

  useAutoResizeTextarea(textareaRef, prompt);

  useEffect(() => {
    setHistoryIndex(-1);
    setDraftBeforeHistory("");
  }, [promptHistory]);

  function updatePromptFromHistory(nextPrompt: string): void {
    onPromptChange(nextPrompt);
    requestAnimationFrame(() => {
      const textarea = textareaRef.current;
      if (!textarea) {
        return;
      }
      const cursor = nextPrompt.length;
      textarea.setSelectionRange(cursor, cursor);
    });
  }

  async function handleSubmit(): Promise<void> {
    const trimmed = prompt.trim();
    if (!trimmed || !agent) {
      return;
    }
    await onSend(trimmed);
    onPromptChange("");
    setHistoryIndex(-1);
    setDraftBeforeHistory("");
  }

  function canNavigateHistory(
    event: KeyboardEvent<HTMLTextAreaElement>,
    direction: "up" | "down",
  ): boolean {
    const textarea = event.currentTarget;
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
      return false;
    }
    if (textarea.selectionStart !== textarea.selectionEnd) {
      return false;
    }

    const beforeCursor = textarea.value.slice(0, textarea.selectionStart);
    const afterCursor = textarea.value.slice(textarea.selectionEnd);

    if (direction === "up") {
      return !beforeCursor.includes("\n");
    }

    return !afterCursor.includes("\n");
  }

  function handleHistoryNavigation(direction: "up" | "down"): void {
    if (promptHistory.length === 0) {
      return;
    }

    if (direction === "up") {
      if (historyIndex === -1) {
        setDraftBeforeHistory(prompt);
        setHistoryIndex(promptHistory.length - 1);
        updatePromptFromHistory(promptHistory[promptHistory.length - 1] ?? "");
        return;
      }

      const nextIndex = Math.max(0, historyIndex - 1);
      setHistoryIndex(nextIndex);
      updatePromptFromHistory(promptHistory[nextIndex] ?? "");
      return;
    }

    if (historyIndex === -1) {
      return;
    }

    const nextIndex = historyIndex + 1;
    if (nextIndex >= promptHistory.length) {
      setHistoryIndex(-1);
      updatePromptFromHistory(draftBeforeHistory);
      return;
    }

    setHistoryIndex(nextIndex);
    updatePromptFromHistory(promptHistory[nextIndex] ?? "");
  }

  function handlePromptKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): void {
    if (event.key === "ArrowUp" && canNavigateHistory(event, "up")) {
      event.preventDefault();
      handleHistoryNavigation("up");
      return;
    }

    if (event.key === "ArrowDown" && canNavigateHistory(event, "down")) {
      event.preventDefault();
      handleHistoryNavigation("down");
      return;
    }

    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }

    event.preventDefault();

    if (sending || stopping || activeRunning || !agent || prompt.trim().length === 0 || sidecarReady !== true) {
      return;
    }

    void handleSubmit();
  }

  const isReviewing = planReviewPhase === "reviewing";
  const isEditing = planReviewPhase === "editing";
  const isSubmittingEdit = planReviewPhase === "submitting_edit";
  const inReviewFlow = isReviewing || isEditing || isSubmittingEdit;
  const composerDisabled = pipelineRunning || inReviewFlow;

  return (
    <div className="bg-surface px-5 pb-2 pt-1">
      <div className="rounded-[20px] border border-edge bg-panel shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        <label className="block">
          <span className="sr-only">Prompt</span>
          <textarea
            ref={textareaRef}
            value={prompt}
            disabled={composerDisabled}
            onChange={(event) => {
              onPromptChange(event.target.value);
              if (historyIndex !== -1) {
                setHistoryIndex(-1);
                setDraftBeforeHistory("");
              }
            }}
            onKeyDown={handlePromptKeyDown}
            rows={1}
            placeholder={pipelineRunning ? "Pipeline is running..." : "Describe the task you want the agent to handle."}
            className={`w-full resize-none bg-transparent px-4 py-3 text-sm leading-6 text-fg placeholder:text-fg-faint focus:outline-none ${composerDisabled ? "cursor-not-allowed opacity-50" : ""}`}
          />
        </label>

        {/* Hide toolbar during any review flow */}
        {inReviewFlow ? null : (
        <div className="flex flex-wrap items-center justify-between gap-2.5 border-t border-edge px-3 py-2.5">
          <div className="flex flex-wrap items-center gap-2">
            <PopoverSelect
              value={pipelineMode}
              options={PIPELINE_OPTIONS}
              disabled={locked}
              direction="up"
              align="left"
              open={openSelect === "pipeline"}
              onOpenChange={(open) => setOpenSelect(open ? "pipeline" : null)}
              triggerClassName="flex h-8 items-center gap-2 rounded-lg border border-edge bg-elevated px-2.5 text-[11px] font-semibold text-fg transition-all hover:border-input-border-focus hover:bg-active disabled:cursor-not-allowed disabled:opacity-55"
              onChange={(value) => onPipelineModeChange(value as PipelineMode)}
            />
            {locked && (
              <span className="inline-flex h-8 items-center rounded-lg border border-edge bg-badge-bg px-2.5 py-1 text-[11px] text-fg-muted">
                Resuming this conversation
              </span>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <div className={`flex items-center gap-1.5 rounded-lg border border-edge bg-surface px-1.5 py-1 shadow-[0_14px_28px_rgba(0,0,0,0.18)] ${pipelineMode !== "simple" ? "invisible" : ""}`}>
                <PopoverSelect
                  value={selectedProviderValue}
                  options={providerOptions}
                  placeholder="Brand"
                  disabled={locked || providerOptions.length === 0}
                  direction="up"
                  align="left"
                  open={openSelect === "provider"}
                  onOpenChange={(open) => setOpenSelect(open ? "provider" : null)}
                  triggerClassName="flex h-7 min-w-[6.75rem] items-center gap-2 rounded-lg border border-edge-strong bg-input-bg px-2.5 text-[11px] font-semibold text-fg shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55"
                  menuClassName="min-w-[11rem] rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
                  onChange={(nextValue) => {
                    const nextProvider = availableProviders.find((provider) => provider.name === nextValue);
                    if (!nextProvider) {
                      return;
                    }
                    onAgentChange({
                      provider: nextProvider.name,
                      model: nextProvider.models[0] ?? "",
                    });
                  }}
                />
                <span className="px-0.5 text-separator">·</span>
                <PopoverSelect
                  value={selectedModelValue}
                  options={modelOptions}
                  placeholder="Model"
                  disabled={locked || modelOptions.length === 0}
                  direction="up"
                  align="right"
                  open={openSelect === "model"}
                  onOpenChange={(open) => setOpenSelect(open ? "model" : null)}
                  triggerClassName="flex h-7 max-w-44 min-w-[7.25rem] items-center gap-2 rounded-lg border border-success-chip-border bg-success-chip-bg px-2.5 text-[11px] font-semibold text-success-chip-text shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-success-chip-border-hover hover:bg-success-chip-bg-hover disabled:cursor-not-allowed disabled:opacity-55"
                  menuClassName="min-w-[12rem] rounded-2xl border border-success-chip-border bg-success-chip-bg p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
                  onChange={(nextValue) => {
                    if (!agent) {
                      return;
                    }
                    onAgentChange({
                      provider: agent.provider,
                      model: nextValue,
                    });
                  }}
                />
            </div>

            {pipelineDone && !pipelineRunning && onResumePipeline && (
              <button
                type="button"
                onClick={onResumePipeline}
                className="inline-flex h-7 items-center gap-1.5 rounded-lg border border-edge bg-elevated px-2.5 text-[11px] font-semibold text-fg transition-colors hover:bg-active"
                title="Resume pipeline"
              >
                <RotateCcw size={10} />
                Resume
              </button>
            )}
            {pipelineDone && !pipelineRunning && onNewPipeline && (
              <button
                type="button"
                onClick={onNewPipeline}
                className="inline-flex h-7 items-center gap-1.5 rounded-lg border border-edge bg-elevated px-2.5 text-[11px] font-semibold text-fg transition-colors hover:bg-active"
                title="Start a new pipeline"
              >
                <Plus size={10} />
                New Pipeline
              </button>
            )}
            <button
              type="button"
              onClick={() => {
                if (activeRunning || pipelineRunning) {
                  void onStop();
                  return;
                }
                void handleSubmit();
              }}
              disabled={(activeRunning || pipelineRunning) ? stopping : stopping || sending || (pipelineMode === "simple" && !agent) || prompt.trim().length === 0 || sidecarReady !== true}
              className={`inline-flex h-7 w-7 items-center justify-center rounded-lg transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${
                activeRunning || pipelineRunning
                  ? "bg-stop-bg text-stop-text hover:bg-stop-bg-hover"
                  : "bg-running-dot text-send-text hover:bg-send-bg-hover"
              }`}
              title={(activeRunning || pipelineRunning) ? (stopping ? "Stopping..." : "Stop") : sidecarReady !== true ? "Waiting for Symphony..." : sending ? "Sending..." : "Send"}
            >
              {(activeRunning || pipelineRunning) ? (
                stopping ? (
                  <span className="text-[10px] font-semibold">...</span>
                ) : (
                  <Square size={12} fill="currentColor" />
                )
              ) : sending ? (
                <span className="text-[10px] font-semibold">...</span>
              ) : (
                <ArrowRight size={12} strokeWidth={2.2} />
              )}
            </button>
          </div>
        </div>
        )}
      </div>
    </div>
  );
}
