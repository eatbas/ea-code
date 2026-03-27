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
  promptHistory: string[];
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
  promptHistory,
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
  const [historyIndex, setHistoryIndex] = useState<number>(-1);
  const [draftBeforeHistory, setDraftBeforeHistory] = useState("");
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

  useEffect(() => {
    setHistoryIndex(-1);
    setDraftBeforeHistory("");
  }, [promptHistory]);

  function updatePromptFromHistory(nextPrompt: string): void {
    setPrompt(nextPrompt);
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
    setPrompt("");
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

    if (sending || activeRunning || !agent || prompt.trim().length === 0) {
      return;
    }

    void handleSubmit();
  }

  return (
    <div className="border-t border-[#313134] bg-[#0b0b0c] px-5 py-4">
      <div className="rounded-[20px] border border-[#313134] bg-[#151516] shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        <label className="block">
          <span className="sr-only">Prompt</span>
          <textarea
            ref={textareaRef}
            value={prompt}
            onChange={(event) => {
              setPrompt(event.target.value);
              if (historyIndex !== -1) {
                setHistoryIndex(-1);
                setDraftBeforeHistory("");
              }
            }}
            onKeyDown={handlePromptKeyDown}
            rows={1}
            placeholder="Describe the task you want the agent to handle."
            className="w-full resize-none bg-transparent px-4 py-3 text-sm leading-6 text-[#f5f5f5] placeholder:text-[#72727a] focus:outline-none"
          />
        </label>

        <div className="flex flex-wrap items-center justify-between gap-2.5 border-t border-[#313134] px-3 py-2.5">
          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex h-8 items-center rounded-full border border-[#313134] bg-[#202022] px-2.5 py-1 text-[11px] font-medium text-[#f5f5f5]">
              Simple Task
            </span>
            {locked && (
              <span className="inline-flex h-8 items-center rounded-full border border-[#313134] bg-[#1c1c1e] px-2.5 py-1 text-[11px] text-[#8b8b93]">
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
            <div className="flex items-center gap-1.5 rounded-full border border-[#313134] bg-[#111112] px-1.5 py-1 shadow-[0_14px_28px_rgba(0,0,0,0.18)]">
              <PopoverSelect
                value={agent?.provider ?? ""}
                options={providerOptions}
                placeholder="Brand"
                disabled={locked || providerOptions.length === 0}
                direction="up"
                align="left"
                open={openSelect === "provider"}
                onOpenChange={(open) => setOpenSelect(open ? "provider" : null)}
                triggerClassName="flex h-7 min-w-[6.75rem] items-center gap-2 rounded-full border border-[#46464b] bg-[#1a1a1c] px-2.5 text-[11px] font-semibold text-[#f5f5f5] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-[#5a5a61] hover:bg-[#202022] disabled:cursor-not-allowed disabled:opacity-55"
                menuClassName="min-w-[11rem] rounded-2xl border border-[#46464b] bg-[#141415] p-1 shadow-[0_20px_44px_rgba(0,0,0,0.38)]"
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
              <span className="px-0.5 text-[#66666d]">·</span>
              <PopoverSelect
                value={agent?.model ?? ""}
                options={modelOptions}
                placeholder="Model"
                disabled={locked || modelOptions.length === 0}
                direction="up"
                align="right"
                open={openSelect === "model"}
                onOpenChange={(open) => setOpenSelect(open ? "model" : null)}
                triggerClassName="flex h-7 max-w-44 min-w-[7.25rem] items-center gap-2 rounded-full border border-[#295638] bg-[#0f1d15] px-2.5 text-[11px] font-semibold text-[#9be7b4] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)] transition-all hover:border-[#37744a] hover:bg-[#13241a] disabled:cursor-not-allowed disabled:opacity-55"
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
              className="inline-flex h-7 w-7 items-center justify-center rounded-lg bg-[#1eb75f] text-[#041107] transition-colors hover:bg-[#2ad16f] disabled:cursor-not-allowed disabled:opacity-50"
              title={sending ? "Sending..." : "Send"}
            >
              {sending ? (
                <span className="text-[10px] font-semibold">...</span>
              ) : (
                <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
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
