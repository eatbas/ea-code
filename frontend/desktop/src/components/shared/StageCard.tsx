import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { StageResult } from "../../types";
import { formatDuration, normaliseDisplayText } from "../../utils/formatters";
import { stageBadgeClass, stageLabel } from "./constants";

interface StageCardProps {
  stage: StageResult;
  modelLabel?: string;
  startedAt?: number;
}

/** Timeline card for a single pipeline stage. Entire card is clickable to toggle content. */
export function StageCard({ stage, modelLabel, startedAt }: StageCardProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [, tick] = useState(0);

  const label = stageLabel(stage.stage);
  const badgeClass = stageBadgeClass(stage.stage);
  const isFailed = stage.status === "failed";
  const isCompleted = stage.status === "completed";
  const isSkipped = stage.status === "skipped";
  const isRunning = stage.status === "running";

  useEffect(() => {
    if (!isRunning || startedAt == null) return;
    const interval = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [isRunning, startedAt]);

  const hasContent = stage.output.length > 0;
  const outputContent = normaliseDisplayText(stage.output);
  const effectiveDurationMs = isRunning && startedAt != null
    ? Math.max(stage.durationMs, Date.now() - startedAt)
    : stage.durationMs;

  return (
    <article
      className={`rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden ${hasContent ? "cursor-pointer" : ""}`}
      onClick={() => hasContent && setOpen((prev) => !prev)}
    >
      {/* Header row */}
      <div className="flex items-center gap-1.5 px-3 py-2 text-[10px] hover:bg-[#1a1a2a] transition-colors">
        {hasContent && (
          <svg
            className={`h-3 w-3 text-[#9898b0] shrink-0 transition-transform ${open ? "rotate-90" : ""}`}
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
        <span
          className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed] ${badgeClass}`}
        >
          {label}
        </span>
        {modelLabel && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {modelLabel}
          </span>
        )}

        {isFailed && <span className="font-medium text-[#ef4444]">Failed</span>}
        {isSkipped && <span className="font-medium text-[#9898b0]">Skipped</span>}

        {/* Right side: time spent + status tag */}
        <div className="ml-auto flex items-center gap-2">
          {effectiveDurationMs > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(effectiveDurationMs)}</span>
          )}
          {isCompleted && (
            <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
              Completed
            </span>
          )}
        </div>
      </div>

      {/* Error message */}
      {stage.error && <p className="px-3 pb-2 text-xs text-[#ef4444]">{stage.error}</p>}

      {/* Expandable content */}
      {open && hasContent && (
        <div className="px-3 pb-3">
          <pre className="overflow-x-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {outputContent}
          </pre>
        </div>
      )}
    </article>
  );
}
