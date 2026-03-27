import type { ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { AgentSelection, ProviderInfo } from "../../types";
import { PopoverSelect } from "../shared/PopoverSelect";
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
  const [openSelect, setOpenSelect] = useState<"provider" | "model" | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const availableProviders = useMemo(
    () => providers.filter((provider) => provider.available),
    [providers],
  );
  const selectedProvider = availableProviders.find((provider) => provider.name === agent?.provider);
  const modelOptions = modelOptionsFromProvider(selectedProvider);
  const providerOptions = useMemo(
    () => availableProviders.map((provider) => ({
      value: provider.name,
      label: providerDisplayName(provider.name),
    })),
    [availableProviders],
  );

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
            <div className="flex items-center gap-2 rounded-full border border-[#2f3448] bg-[#111522] px-2 py-2 shadow-[0_14px_28px_rgba(0,0,0,0.18)]">
              <PopoverSelect
                value={agent?.provider ?? ""}
                options={providerOptions}
                placeholder="Brand"
                disabled={locked || providerOptions.length === 0}
                direction="up"
                align="left"
                open={openSelect === "provider"}
                onOpenChange={(open) => setOpenSelect(open ? "provider" : null)}
                triggerClassName="flex h-9 min-w-[8.5rem] items-center gap-2 rounded-full border border-[#363b52] bg-[#181c2a] px-3 text-xs font-semibold text-[#f4f6fd] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-[#4a5273] hover:bg-[#1d2232] disabled:cursor-not-allowed disabled:opacity-55"
                menuClassName="min-w-[11rem] rounded-2xl border border-[#363b52] bg-[#141925] p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
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
              <span className="text-[#4f587a]">·</span>
              <PopoverSelect
                value={agent?.model ?? ""}
                options={modelOptions}
                placeholder="Model"
                disabled={locked || modelOptions.length === 0}
                direction="up"
                align="right"
                open={openSelect === "model"}
                onOpenChange={(open) => setOpenSelect(open ? "model" : null)}
                triggerClassName="flex h-9 max-w-52 min-w-[9rem] items-center gap-2 rounded-full border border-[#295638] bg-[#0f1d15] px-3 text-xs font-semibold text-[#9be7b4] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-[#37744a] hover:bg-[#13241a] disabled:cursor-not-allowed disabled:opacity-55"
                menuClassName="min-w-[12rem] rounded-2xl border border-[#295638] bg-[#101a14] p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
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
