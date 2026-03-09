import { useState } from "react";
import type { ReactNode } from "react";
import { formatDuration } from "../../utils/formatters";

interface FinalPlanCardProps {
  plannerPlan?: string;
  auditedPlan?: string;
  /** Combined duration of planning + auditing stages in milliseconds. */
  durationMs?: number;
}

/** Collapsible card showing combined plan + audited plan with timing and status. */
export function FinalPlanCard({ plannerPlan, auditedPlan, durationMs }: FinalPlanCardProps): ReactNode {
  const [open, setOpen] = useState(false);

  return (
    <article
      className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden cursor-pointer"
      onClick={() => setOpen((prev) => !prev)}
    >
      <div className="flex items-center gap-2 px-3 py-2 hover:bg-[#1a1a2a] transition-colors">
        <svg
          className={`h-3 w-3 text-[#9898b0] shrink-0 transition-transform ${open ? "rotate-90" : ""}`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: "rgba(64, 196, 255, 0.24)" }}
        >
          Final Plan
        </span>

        {/* Right side: combined duration + completed tag */}
        <div className="ml-auto flex items-center gap-2 text-[10px]">
          {durationMs != null && durationMs > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(durationMs)}</span>
          )}
          <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
            Completed
          </span>
        </div>
      </div>
      {open && (
        <div className="flex flex-col gap-3 px-3 pb-3">
          {plannerPlan && (
            <div>
              <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                Plan
              </span>
              <pre className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed break-words">
                {plannerPlan}
              </pre>
            </div>
          )}
          {auditedPlan && (
            <div>
              <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                Audited Plan
              </span>
              <pre className="rounded border border-[#3b82f6]/20 bg-[#3b82f6]/5 px-3 py-2 text-xs text-[#e4e4ed] whitespace-pre-wrap leading-relaxed break-words">
                {auditedPlan}
              </pre>
            </div>
          )}
        </div>
      )}
    </article>
  );
}
