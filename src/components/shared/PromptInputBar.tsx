import type { ReactNode } from "react";
import { useState, useRef, useEffect } from "react";
import type { AgentBackend, RunOptions, CliHealth, AppSettings } from "../../types";
import { CLI_MODEL_OPTIONS } from "../../types";
import { hasMinimumAgentsConfigured } from "../../utils/agentSettings";
import { BACKEND_OPTIONS } from "./constants";
import { PopoverSelect } from "./PopoverSelect";

interface PromptInputBarProps {
  placeholder?: string;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup?: () => void;
  onSubmit: (options: RunOptions) => void;
}

/** Shared prompt input bar with textarea, direct task checkbox, and agent/model selectors. */
export function PromptInputBar({
  placeholder,
  cliHealth,
  settings,
  onMissingAgentSetup,
  onSubmit,
}: PromptInputBarProps): ReactNode {
  const [prompt, setPrompt] = useState<string>("");
  const [directTask, setDirectTask] = useState(false);
  const [noPlan, setNoPlan] = useState(false);
  const [directAgent, setDirectAgent] = useState<AgentBackend>("claude");
  const [directModel, setDirectModel] = useState<string>("sonnet");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const availableBackends = BACKEND_OPTIONS.filter(
    (opt) => cliHealth?.[opt.value]?.available,
  );

  const hasPrompt = prompt.trim().length > 0;
  const missingMinimumAgents = !settings || !hasMinimumAgentsConfigured(settings);
  const blockedByMinimumAgents = !directTask && missingMinimumAgents;
  const canRun = hasPrompt && !blockedByMinimumAgents;

  const disabledReason = blockedByMinimumAgents
    ? "Go to Settings/Agents and set the minimum agent roles before sending."
    : directTask
      ? "Run direct task"
      : "Run pipeline";

  useEffect(() => {
    const el = textareaRef.current;
    if (el) { el.style.height = "auto"; el.style.height = `${Math.min(el.scrollHeight, 160)}px`; }
  }, [prompt]);

  function handleSubmit(): void {
    if (blockedByMinimumAgents) {
      onMissingAgentSetup?.();
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
          className="flex-1 resize-none bg-transparent text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:outline-none"
          style={{ maxHeight: "160px" }}
        />
        <span
          className="shrink-0"
          title={disabledReason}
          onClick={() => {
            if (blockedByMinimumAgents) {
              onMissingAgentSetup?.();
            }
          }}
        >
          <button
            onClick={handleSubmit}
            disabled={!canRun}
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
                const opts = CLI_MODEL_OPTIONS[first];
                if (opts && opts.length > 0) setDirectModel(opts[0].value);
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
                const backend = val as AgentBackend;
                setDirectAgent(backend);
                const opts = CLI_MODEL_OPTIONS[backend];
                if (opts && opts.length > 0) setDirectModel(opts[0].value);
              }}
            />
            <PopoverSelect
              value={directModel}
              options={CLI_MODEL_OPTIONS[directAgent] ?? []}
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
