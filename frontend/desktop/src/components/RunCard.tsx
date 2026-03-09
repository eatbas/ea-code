import type { ReactNode } from "react";
import type { RunDetail, StageEntry, PipelineStage } from "../types";
import { formatDuration, formatTimestamp } from "../utils/formatters";
import { STAGE_LABELS, STAGE_COLOURS } from "./shared/constants";
import { PromptCard } from "./shared/PromptCard";

interface RunCardProps {
  run: RunDetail;
}

/** Collapsible stage row for a historical stage entry. */
function HistoryStageCard({ entry }: { entry: StageEntry }): ReactNode {
  const label = STAGE_LABELS[entry.stage as PipelineStage] ?? entry.stage;
  const badgeBg = STAGE_COLOURS[entry.stage as PipelineStage] ?? "rgba(150,150,150,0.22)";
  const isFailed = entry.status === "failed";
  const hasOutput = entry.output && entry.output.trim().length > 0;

  return (
    <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] px-3 py-2">
      <div className="flex flex-wrap items-center gap-1.5 text-[10px]">
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: badgeBg }}
        >
          {label}
        </span>
        {entry.durationMs > 0 && (
          <span className="text-[#9898b0] opacity-80">
            {formatDuration(entry.durationMs)}
          </span>
        )}
        {isFailed && (
          <span className="font-medium text-[#ef4444]">Failed</span>
        )}
        {entry.status === "skipped" && (
          <span className="font-medium text-[#9898b0]">Skipped</span>
        )}
      </div>
      {entry.error && (
        <p className="mt-1.5 text-xs text-[#ef4444]">{entry.error}</p>
      )}
      {hasOutput && (
        <details className="mt-1.5">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70 hover:opacity-100 transition-opacity">
            Details
          </summary>
          <pre className="mt-1.5 max-h-64 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {entry.output}
          </pre>
        </details>
      )}
    </article>
  );
}

/** Displays a single historical run with full step-by-step timeline. */
export function RunCard({ run }: RunCardProps): ReactNode {
  const statusColour = run.status === "completed"
    ? "#22c55e"
    : run.status === "failed"
      ? "#ef4444"
      : run.status === "cancelled"
        ? "#f59e0b"
        : "#9898b0";

  const totalDurationMs = run.startedAt && run.completedAt
    ? new Date(run.completedAt).getTime() - new Date(run.startedAt).getTime()
    : 0;

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt — right-aligned bubble */}
      <div className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
          {run.prompt}
        </div>
      </div>

      {/* Prompt received */}
      <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden">
        <details>
          <summary className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-[#1a1a2a] transition-colors">
            <svg
              className="h-3 w-3 text-[#9898b0] transition-transform [details[open]>&]:rotate-90"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M8 5v14l11-7z" />
            </svg>
            <span
              className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
              style={{ background: "rgba(34, 197, 94, 0.22)" }}
            >
              Prompt Received
            </span>
          </summary>
          <div className="px-3 pb-3">
            <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
              {run.prompt}
            </div>
          </div>
        </details>
      </article>

      {/* Iteration stages */}
      {run.iterations.map((iter) => (
        <div key={iter.number} className="flex flex-col gap-2">
          {/* Stage cards */}
          {iter.stages.map((entry) => (
            <div key={entry.id} className="flex flex-col gap-2">
              <HistoryStageCard entry={entry} />

              {/* After prompt_enhance — show enhanced prompt comparison */}
              {entry.stage === "prompt_enhance" && entry.status === "completed" && iter.enhancedPrompt && (
                <PromptCard originalPrompt={run.prompt} enhancedPrompt={iter.enhancedPrompt} />
              )}

              {/* After plan_audit — show final plan */}
              {entry.stage === "plan_audit" && entry.status === "completed" && iter.auditedPlan && (
                <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden">
                  <details>
                    <summary className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-[#1a1a2a] transition-colors">
                      <svg
                        className="h-3 w-3 text-[#9898b0] transition-transform [details[open]>&]:rotate-90"
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
                    </summary>
                    <div className="px-3 pb-3">
                      <pre className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#e4e4ed] whitespace-pre-wrap leading-relaxed break-words">
                        {iter.auditedPlan}
                      </pre>
                    </div>
                  </details>
                </article>
              )}
            </div>
          ))}
        </div>
      ))}

      {/* Result summary */}
      <div
        className="rounded-lg border px-3 py-2"
        style={{
          background: run.status === "completed" ? "rgba(40,180,95,0.10)" : run.status === "failed" ? "rgba(230,75,75,0.10)" : "#1a1a24",
          borderColor: run.status === "completed" ? "rgba(40,180,95,0.30)" : run.status === "failed" ? "rgba(230,75,75,0.30)" : "#2e2e48",
        }}
      >
        <div className="flex items-center gap-2">
          <div className="h-2 w-2 rounded-full" style={{ backgroundColor: statusColour }} />
          <span className="text-xs font-medium capitalize" style={{ color: statusColour }}>
            {run.status}
          </span>
          {run.finalVerdict && (
            <span className="text-[10px] px-1.5 py-0.5 rounded uppercase font-semibold" style={{ color: statusColour, backgroundColor: `${statusColour}15` }}>
              {run.finalVerdict}
            </span>
          )}
          <div className="ml-auto flex items-center gap-2 text-[11px] text-[#6f7086]">
            {run.iterations.length > 0 && (
              <span>{run.iterations.length} {run.iterations.length === 1 ? "iteration" : "iterations"}</span>
            )}
            {totalDurationMs > 0 && (
              <span>{formatDuration(totalDurationMs)}</span>
            )}
            {run.completedAt && (
              <span>{formatTimestamp(run.completedAt)}</span>
            )}
          </div>
        </div>
        {run.executiveSummary && (
          <p className="mt-2 text-xs text-[#c4c4d4] whitespace-pre-wrap leading-relaxed">
            {run.executiveSummary}
          </p>
        )}
        {run.error && (
          <p className="mt-1.5 text-xs text-[#ef4444]">{run.error}</p>
        )}
      </div>
    </div>
  );
}
