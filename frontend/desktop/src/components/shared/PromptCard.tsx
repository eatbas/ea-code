import { useState } from "react";
import type { ReactNode } from "react";

interface PromptCardProps {
  originalPrompt: string;
  enhancedPrompt: string;
}

/** Collapsible card showing the original and enhanced prompts side by side. */
export function PromptCard({ originalPrompt, enhancedPrompt }: PromptCardProps): ReactNode {
  const [open, setOpen] = useState(true);

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
        <span className="text-[10px] font-semibold uppercase tracking-widest text-[#4ade80]">
          Enhanced Prompt
        </span>
      </button>

      {open && (
        <div className="flex flex-col gap-3 px-3 pb-3">
          {/* Original prompt */}
          <div>
            <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
              Original
            </span>
            <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
              {originalPrompt}
            </div>
          </div>

          {/* Enhanced prompt */}
          <div>
            <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
              Enhanced
            </span>
            <div className="rounded border border-[#22c55e]/20 bg-[#22c55e]/5 px-3 py-2 text-xs text-[#e4e4ed] whitespace-pre-wrap leading-relaxed">
              {enhancedPrompt}
            </div>
          </div>
        </div>
      )}
    </article>
  );
}
