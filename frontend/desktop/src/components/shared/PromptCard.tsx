import { useState } from "react";
import type { ReactNode } from "react";
import { formatDuration } from "../../utils/formatters";

interface PromptCardProps {
  originalPrompt: string;
  enhancedPrompt: string;
  /** Duration in milliseconds from the prompt_enhance stage. */
  durationMs?: number;
}

type PromptTab = "input" | "output";

/** Collapsible card with Input/Output tabs for original and enhanced prompts. */
export function PromptCard({ originalPrompt, enhancedPrompt, durationMs }: PromptCardProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<PromptTab>("output");

  return (
    <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden">
      {/* Header — clickable to toggle */}
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-[#1a1a2a] transition-colors"
      >
        <svg
          className={`h-3 w-3 text-[#9898b0] transition-transform ${open ? "rotate-90" : ""}`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span
          className="rounded bg-[#22c55e]/20 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
        >
          Enhanced Prompt
        </span>

        {/* Right side: duration + completed tag */}
        <div className="ml-auto flex items-center gap-2 text-[10px]">
          {durationMs != null && durationMs > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(durationMs)}</span>
          )}
          <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
            Completed
          </span>
        </div>
      </button>

      {open && (
        <div className="px-3 pb-3">
          {/* Tabs */}
          <div className="flex gap-1 mb-2">
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setTab("input"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                tab === "input"
                  ? "bg-[#9898b0]/20 text-[#e4e4ed]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Input
            </button>
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setTab("output"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                tab === "output"
                  ? "bg-[#22c55e]/20 text-[#4ade80]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Output
            </button>
          </div>

          {/* Tab content */}
          {tab === "input" && (
            <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
              {originalPrompt}
            </div>
          )}
          {tab === "output" && (
            <div className="rounded border border-[#22c55e]/20 bg-[#22c55e]/5 px-3 py-2 text-xs text-[#e4e4ed] whitespace-pre-wrap leading-relaxed">
              {enhancedPrompt}
            </div>
          )}
        </div>
      )}
    </article>
  );
}
