import { useState } from "react";
import type { ReactNode } from "react";

interface PromptCardProps {
  originalPrompt: string;
  enhancedPrompt: string;
}

type PromptTab = "input" | "output";

/** Collapsible card with Input/Output tabs for original and enhanced prompts. */
export function PromptCard({ originalPrompt, enhancedPrompt }: PromptCardProps): ReactNode {
  const [open, setOpen] = useState(true);
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
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: "rgba(34, 197, 94, 0.22)" }}
        >
          Enhanced Prompt
        </span>
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
