import type { ReactNode } from "react";
import type { StageResult } from "../../types";
import { formatDuration } from "../../utils/formatters";
import { STAGE_LABELS, STAGE_COLOURS } from "./constants";

interface StageCardProps {
  stage: StageResult;
  logs?: string[];
}

/** Timeline card for a single completed pipeline stage. */
export function StageCard({ stage, logs }: StageCardProps): ReactNode {
  const label = STAGE_LABELS[stage.stage] ?? stage.stage;
  const badgeBg = STAGE_COLOURS[stage.stage] ?? "rgba(150,150,150,0.22)";
  const isFailed = stage.status === "failed";

  const logLines = logs ?? [];
  const hasContent = logLines.length > 0 || (stage.output && stage.output.length > 0);

  return (
    <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] px-3 py-2">
      {/* Meta row */}
      <div className="flex flex-wrap items-center gap-1.5 text-[10px]">
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: badgeBg }}
        >
          {label}
        </span>

        {stage.durationMs > 0 && (
          <span className="text-[#9898b0] opacity-80">
            {formatDuration(stage.durationMs)}
          </span>
        )}

        {isFailed && (
          <span className="font-medium text-[#ef4444]">Failed</span>
        )}
      </div>

      {/* Error message */}
      {stage.error && (
        <p className="mt-1.5 text-xs text-[#ef4444]">{stage.error}</p>
      )}

      {/* Collapsible details */}
      {hasContent && (
        <details className="mt-1.5">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70 hover:opacity-100 transition-opacity">
            {logLines.length > 0 ? `Agent logs (${logLines.length} lines)` : "Details"}
          </summary>
          <pre className="mt-1.5 max-h-64 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {logLines.length > 0 ? logLines.join("\n") : stage.output}
          </pre>
        </details>
      )}
    </article>
  );
}
