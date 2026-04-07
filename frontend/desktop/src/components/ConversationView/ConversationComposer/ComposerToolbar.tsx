import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { ArrowRight, Plus, RotateCcw, Square } from "lucide-react";
import type { AgentSelection, ProviderInfo } from "../../../types";
import { PopoverSelect } from "../../shared/PopoverSelect";
import { modelOptionsFromProvider, providerDisplayName, THINKING_TRIGGER_LABELS } from "../../shared/constants";
import type { PipelineMode } from "./index";

const PIPELINE_OPTIONS: { value: PipelineMode; label: string }[] = [
  { value: "auto", label: "Auto" },
  { value: "simple", label: "Simple Task" },
  { value: "code", label: "Code Pipeline" },
];

interface ComposerToolbarProps {
  providers: ProviderInfo[];
  agent: AgentSelection | null;
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
  onPipelineModeChange: (mode: PipelineMode) => void;
  onAgentChange: (agent: AgentSelection) => void;
  onThinkingChange: (value: string) => void;
  onSubmit: () => Promise<void>;
  onStop: () => Promise<void>;
  onResumePipeline?: () => void;
  onNewPipeline?: () => void;
  submitDisabled: boolean;
}

export function ComposerToolbar({
  providers,
  agent,
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
  onPipelineModeChange,
  onAgentChange,
  onThinkingChange,
  onSubmit,
  onStop,
  onResumePipeline,
  onNewPipeline,
  submitDisabled,
}: ComposerToolbarProps): ReactNode {
  const [openSelect, setOpenSelect] = useState<"pipeline" | "provider" | "model" | "thinking" | null>(null);
  const hasThinking = thinkingOptions !== undefined && thinkingOptions.length > 0;
  const triggerLabels = agent ? THINKING_TRIGGER_LABELS[agent.provider] : undefined;
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

  return (
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
          {hasThinking && (
            <>
              <span className="px-0.5 text-separator">·</span>
              <PopoverSelect
                value={thinkingLevel}
                options={thinkingOptions}
                placeholder="Default"
                disabled={locked}
                direction="up"
                align="right"
                open={openSelect === "thinking"}
                onOpenChange={(open) => setOpenSelect(open ? "thinking" : null)}
                triggerClassName="flex h-7 w-[5rem] items-center gap-1 rounded-lg border border-edge-strong bg-input-bg px-2 text-[11px] font-semibold text-fg shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55"
                menuClassName="w-max min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
                triggerLabels={triggerLabels}
                onChange={onThinkingChange}
              />
            </>
          )}
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
            void onSubmit();
          }}
          disabled={(activeRunning || pipelineRunning) ? stopping : submitDisabled}
          className={`inline-flex h-7 w-7 items-center justify-center rounded-lg transition-colors disabled:cursor-not-allowed disabled:opacity-50 ${
            activeRunning || pipelineRunning
              ? "bg-stop-bg text-stop-text hover:bg-stop-bg-hover"
              : "bg-running-dot text-send-text hover:bg-send-bg-hover"
          }`}
          title={(activeRunning || pipelineRunning)
            ? (stopping ? "Stopping..." : "Stop")
            : sidecarReady !== true
              ? "Waiting for Symphony..."
              : sending
                ? "Sending..."
                : "Send"}
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
  );
}
