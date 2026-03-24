import type { ReactNode } from "react";
import { useState, useRef, useEffect } from "react";
import type { RunOptions, AppSettings, ProviderInfo } from "../../types";
import {
  hasMinimumAgentsConfigured,
  missingMinimumAgentModelLabels,
} from "../../utils/agentSettings";
import {
  backendOptionsFromProviders,
  modelOptionsFromProvider,
} from "./constants";
import { PopoverSelect } from "./PopoverSelect";
import { useToast } from "./Toast";

interface PromptInputBarProps {
  placeholder?: string;
  providers: ProviderInfo[];
  settings: AppSettings | null;
  hasProjectSelected?: boolean;
  onMissingAgentSetup?: () => void;
  onSubmit: (options: RunOptions) => void;
}

/** Shared prompt input bar with textarea, direct task checkbox, and agent/model selectors. */
export function PromptInputBar({
  placeholder,
  providers,
  settings,
  hasProjectSelected = true,
  onMissingAgentSetup,
  onSubmit,
}: PromptInputBarProps): ReactNode {
  const toast = useToast();
  const [prompt, setPrompt] = useState<string>("");
  const [directTask, setDirectTask] = useState(false);
  const [noPlan, setNoPlan] = useState(false);
  const [directAgent, setDirectAgent] = useState<string>("claude");
  const [directModel, setDirectModel] = useState<string>("sonnet");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const providerList = providers ?? [];
  const availableBackends = backendOptionsFromProviders(
    providerList.filter((p) => p.available),
  );

  const hasPrompt = prompt.trim().length > 0;
  const missingProject = !hasProjectSelected;
  const missingMinimumAgents = !settings || !hasMinimumAgentsConfigured(settings);
  const missingMinimumAgentModels = settings
    ? missingMinimumAgentModelLabels(settings).length > 0
    : true;
  const blockedByMinimumAgents = !directTask && (missingMinimumAgents || missingMinimumAgentModels);
  const canRun = hasPrompt && !missingProject && !blockedByMinimumAgents;
  const canAttemptSubmit = hasPrompt;

  let disabledReason = directTask ? "Run direct task" : "Run pipeline";
  if (!hasPrompt) {
    disabledReason = "Type a prompt before sending.";
  } else if (missingProject && blockedByMinimumAgents) {
    disabledReason = missingMinimumAgents
      ? "No project selected and no agents selected. Select a project and configure required agent roles before sending."
      : "No project selected and agent models are not selected. Select a project and configure required models before sending.";
  } else if (missingProject) {
    disabledReason = "No project selected. Select a project before sending.";
  } else if (blockedByMinimumAgents) {
    disabledReason = missingMinimumAgentModels
      ? "Agent models are not selected. Go to Settings/CLI Setup and choose models before sending."
      : "No agents selected. Go to Settings/Agents and set the required agent roles before sending.";
  }

  useEffect(() => {
    const el = textareaRef.current;
    if (el) { el.style.height = "auto"; el.style.height = `${Math.min(el.scrollHeight, 160)}px`; }
  }, [prompt]);

  function showMissingSetupToast(): void {
    if (missingProject) {
      toast.error("Please select a project.");
      return;
    }
    if (missingMinimumAgents || missingMinimumAgentModels) {
      toast.error("Please select the agents under Settings/Agents.", onMissingAgentSetup
        ? { label: "Open Settings/Agents", onClick: onMissingAgentSetup }
        : undefined);
    }
  }

  function handleSubmit(): void {
    if (missingProject || blockedByMinimumAgents) {
      showMissingSetupToast();
      return;
    }

    if (canRun) {
      onSubmit({
        prompt,
        directTask,
        directTaskAgent: directTask ? directAgent : undefined,
        directTaskModel: directTask ? directModel : undefined,
        noPlan,
      });
      setPrompt("");
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function getModelOptionsForAgent(agent: string): { value: string; label: string }[] {
    const provider = providerList.find((p) => p.name === agent);
    return modelOptionsFromProvider(provider);
  }

  return (
    <div className="flex w-full flex-col rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3 gap-2">
      {/* Top row: textarea + submit */}
      <div className="flex items-center gap-2">
        <textarea
          ref={textareaRef}
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder ?? "What would you like to build?"}
          rows={1}
          className="max-h-40 flex-1 resize-none bg-transparent text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:outline-none"
        />
        <span
          className="shrink-0"
          title={!canRun ? disabledReason : undefined}
        >
          <button
            onClick={handleSubmit}
            disabled={!canAttemptSubmit}
            className="rounded-lg bg-[#e4e4ed] p-2 text-[#0f0f14] hover:bg-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title={canRun ? disabledReason : undefined}
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="19" x2="12" y2="5" />
              <polyline points="5 12 12 5 19 12" />
            </svg>
          </button>
        </span>
      </div>

      {/* Bottom row: checkboxes + dropdowns */}
      <div className="flex items-center gap-4 border-t border-[#2e2e48] pt-2">
        <label className="flex items-center gap-1.5 text-xs text-[#9898b0] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={directTask}
            onChange={(e) => {
              const checked = e.target.checked;
              setDirectTask(checked);
              if (checked && availableBackends.length > 0) {
                const first = availableBackends[0].value;
                setDirectAgent(first);
                const opts = getModelOptionsForAgent(first);
                if (opts.length > 0) setDirectModel(opts[0].value);
              }
            }}
            className="h-3.5 w-3.5 rounded border-[#2e2e48] bg-[#1a1a24] accent-[#6366f1]"
          />
          Direct Task
        </label>
        <label className="flex items-center gap-1.5 text-xs text-[#9898b0] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={noPlan}
            onChange={(e) => setNoPlan(e.target.checked)}
            disabled={directTask}
            className="h-3.5 w-3.5 rounded border-[#2e2e48] bg-[#1a1a24] accent-[#6366f1] disabled:opacity-30"
          />
          <span className={directTask ? "opacity-30" : ""}>No Plan</span>
        </label>
        {directTask && (
          <div className="ml-auto flex items-center gap-2">
            <PopoverSelect
              value={directAgent}
              options={availableBackends}
              direction="up"
              align="left"
              onChange={(val) => {
                setDirectAgent(val);
                const opts = getModelOptionsForAgent(val);
                if (opts.length > 0) setDirectModel(opts[0].value);
              }}
            />
            <PopoverSelect
              value={directModel}
              options={getModelOptionsForAgent(directAgent)}
              direction="up"
              align="right"
              onChange={setDirectModel}
            />
          </div>
        )}
      </div>
    </div>
  );
}
