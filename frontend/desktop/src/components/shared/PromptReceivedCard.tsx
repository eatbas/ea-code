import { useState } from "react";
import type { ReactNode } from "react";

interface PromptReceivedCardProps {
  prompt: string;
}

/** Collapsible card showing the prompt captured for the run. */
export function PromptReceivedCard({ prompt }: PromptReceivedCardProps): ReactNode {
  const [open, setOpen] = useState(false);

  return (
    <article
      className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden cursor-pointer"
      onClick={() => setOpen((prev) => !prev)}
    >
      <div className="flex items-center gap-2 px-3 py-2 hover:bg-[#1a1a2a] transition-colors">
        <svg
          className={`h-3 w-3 text-[#9898b0] transition-transform ${open ? "rotate-90" : ""}`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span className="rounded bg-[#22c55e]/20 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]">
          Prompt Received
        </span>
        <span className="ml-auto rounded bg-[#22c55e]/10 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e]">
          Completed
        </span>
      </div>
      {open && (
        <div className="px-3 pb-3">
          <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
            {prompt}
          </div>
        </div>
      )}
    </article>
  );
}
