import type { ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { AgentSelection, ProviderInfo } from "../../types";
import {
  modelOptionsFromProvider,
  providerDisplayName,
} from "../shared/constants";

interface ConversationComposerProps {
  providers: ProviderInfo[];
  agent: AgentSelection | null;
  locked: boolean;
  sending: boolean;
  stopping: boolean;
  activeRunning: boolean;
  onAgentChange: (agent: AgentSelection) => void;
  onSend: (prompt: string) => Promise<void>;
  onStop: () => Promise<void>;
}

export function ConversationComposer({
  providers,
  agent,
  locked,
  sending,
  stopping,
  activeRunning,
  onAgentChange,
  onSend,
  onStop,
}: ConversationComposerProps): ReactNode {
  const [prompt, setPrompt] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const availableProviders = useMemo(
    () => providers.filter((provider) => provider.available),
    [providers],
  );
  const selectedProvider = availableProviders.find((provider) => provider.name === agent?.provider);
  const modelOptions = modelOptionsFromProvider(selectedProvider);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) {
      return;
    }

    textarea.style.height = "0px";

    const computedStyle = window.getComputedStyle(textarea);
    const lineHeight = Number.parseFloat(computedStyle.lineHeight) || 24;
    const paddingTop = Number.parseFloat(computedStyle.paddingTop) || 0;
    const paddingBottom = Number.parseFloat(computedStyle.paddingBottom) || 0;
    const maxHeight = lineHeight * 3 + paddingTop + paddingBottom;
    const nextHeight = Math.min(textarea.scrollHeight, maxHeight);

    textarea.style.height = `${nextHeight}px`;
    textarea.style.overflowY = textarea.scrollHeight > maxHeight ? "auto" : "hidden";
  }, [prompt]);

  async function handleSubmit(): Promise<void> {
    const trimmed = prompt.trim();
    if (!trimmed || !agent) {
      return;
    }
    await onSend(trimmed);
    setPrompt("");
  }

  function handlePromptKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): void {
    if (event.key !== "Enter" || event.shiftKey) {
      return;
    }

    event.preventDefault();

    if (sending || activeRunning || !agent || prompt.trim().length === 0) {
      return;
    }

    void handleSubmit();
  }

  return (
    <div className="border-t border-[#2e2e48] bg-[#0f0f14] px-5 py-4">
      <div className="rounded-[20px] border border-[#2e2e48] bg-[#1a1a24] shadow-[0_0_0_1px_rgba(46,46,72,0.24)]">
        <label className="block">
          <span className="sr-only">Prompt</span>
          <textarea
            ref={textareaRef}
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            onKeyDown={handlePromptKeyDown}
            rows={1}
            placeholder="Describe the task you want the agent to handle."
            className="w-full resize-none bg-transparent px-4 py-3 text-sm leading-6 text-[#e4e4ed] placeholder:text-[#6b6b80] focus:outline-none"
          />
        </label>

        <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[#2e2e48] px-3 py-3">
          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex items-center rounded-full border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs font-medium text-[#e4e4ed]">
              Simple Task
            </span>
            {locked && (
              <span className="inline-flex items-center rounded-full border border-[#2e2e48] bg-[#202031] px-3 py-1.5 text-xs text-[#9898b0]">
                Resuming this conversation
              </span>
            )}
            {activeRunning && (
              <button
                type="button"
                onClick={() => {
                  void onStop();
                }}
                disabled={stopping}
                className="rounded-full border border-[#4f1f22] bg-[#211112] px-3 py-1.5 text-xs font-medium text-[#f2b7b7] transition-colors hover:bg-[#2a1416] disabled:cursor-not-allowed disabled:opacity-60"
              >
                {stopping ? "Stopping..." : "Stop"}
              </button>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <div className="flex items-center gap-2 rounded-full border border-[#2e2e48] bg-[#24243a] px-3 py-1.5">
              <select
                value={agent?.provider ?? ""}
                disabled={locked || availableProviders.length === 0}
                onChange={(event) => {
                  const nextProvider = availableProviders.find((provider) => provider.name === event.target.value);
                  if (!nextProvider) {
                    return;
                  }
                  onAgentChange({
                    provider: nextProvider.name,
                    model: nextProvider.models[0] ?? "",
                  });
                }}
                className={`bg-transparent text-xs font-medium text-[#e4e4ed] focus:outline-none ${
                  locked || availableProviders.length === 0 ? "disabled:cursor-not-allowed disabled:opacity-60" : "cursor-pointer"
                }`}
              >
                <option value="" disabled>Select provider</option>
                {availableProviders.map((provider) => (
                  <option key={provider.name} value={provider.name} className="bg-[#1a1a24] text-[#e4e4ed]">
                    {providerDisplayName(provider.name)}
                  </option>
                ))}
              </select>
              <span className="text-[#5f6378]">·</span>
              <select
                value={agent?.model ?? ""}
                disabled={locked || modelOptions.length === 0}
                onChange={(event) => {
                  if (!agent) {
                    return;
                  }
                  onAgentChange({
                    provider: agent.provider,
                    model: event.target.value,
                  });
                }}
                className={`max-w-44 rounded-full border border-[#295638] bg-[#0d1811] px-2 py-1 text-xs font-medium text-[#8ce6a8] focus:outline-none ${
                  locked || modelOptions.length === 0 ? "disabled:cursor-not-allowed disabled:opacity-60" : "cursor-pointer"
                }`}
              >
                <option value="" disabled>Model</option>
                {modelOptions.map((model) => (
                  <option key={model.value} value={model.value} className="bg-[#111512] text-[#ecfdf3]">
                    {model.label}
                  </option>
                ))}
              </select>
            </div>

            <button
              type="button"
              onClick={() => {
                void handleSubmit();
              }}
              disabled={sending || activeRunning || !agent || prompt.trim().length === 0}
              className="inline-flex h-9 w-9 items-center justify-center rounded-xl bg-[#1eb75f] text-[#041107] transition-colors hover:bg-[#2ad16f] disabled:cursor-not-allowed disabled:opacity-50"
              title={sending ? "Sending..." : "Send"}
            >
              {sending ? (
                <span className="text-[10px] font-semibold">...</span>
              ) : (
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M5 12h14" />
                  <path d="m12 5 7 7-7 7" />
                </svg>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
