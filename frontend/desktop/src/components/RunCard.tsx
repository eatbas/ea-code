import { useState } from "react";
import type { ReactNode } from "react";
import type { RunDetail, StageEntry, PipelineStage } from "../types";
import { formatDuration } from "../utils/formatters";
import { STAGE_LABELS, STAGE_COLOURS } from "./shared/constants";
import { PromptCard } from "./shared/PromptCard";
import { FinalPlanCard } from "./shared/FinalPlanCard";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";

interface RunCardProps {
  run: RunDetail;
}

/** Clickable stage row for a historical stage entry. Whole card toggles content. */
function HistoryStageCard({ entry }: { entry: StageEntry }): ReactNode {
  const [open, setOpen] = useState(false);

  const label = STAGE_LABELS[entry.stage as PipelineStage] ?? entry.stage;
  const badgeBg = STAGE_COLOURS[entry.stage as PipelineStage] ?? "rgba(150,150,150,0.22)";
  const isFailed = entry.status === "failed";
  const isCompleted = entry.status === "completed";
  const hasOutput = entry.output && entry.output.trim().length > 0;

  return (
    <article
      className={`rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden ${hasOutput ? "cursor-pointer" : ""}`}
      onClick={() => hasOutput && setOpen((prev) => !prev)}
    >
      <div className="flex items-center gap-1.5 px-3 py-2 text-[10px] hover:bg-[#1a1a2a] transition-colors">
        {hasOutput && (
          <svg
            className={`h-3 w-3 text-[#9898b0] shrink-0 transition-transform ${open ? "rotate-90" : ""}`}
            viewBox="0 0 24 24"
            fill="currentColor"
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: badgeBg }}
        >
          {label}
        </span>
        {isFailed && <span className="font-medium text-[#ef4444]">Failed</span>}
        {entry.status === "skipped" && <span className="font-medium text-[#9898b0]">Skipped</span>}
        <div className="ml-auto flex items-center gap-2">
          {entry.durationMs > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(entry.durationMs)}</span>
          )}
          {isCompleted && (
            <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
              Completed
            </span>
          )}
        </div>
      </div>
      {entry.error && <p className="px-3 pb-2 text-xs text-[#ef4444]">{entry.error}</p>}
      {open && hasOutput && (
        <div className="px-3 pb-3">
          <pre className="max-h-64 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {entry.output}
          </pre>
        </div>
      )}
    </article>
  );
}

/** Collapsible Prompt Received card matching ChatView style. */
function PromptReceivedCard({ prompt }: { prompt: string }): ReactNode {
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
        <span
          className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
          style={{ background: "rgba(34, 197, 94, 0.22)" }}
        >
          Prompt Received
        </span>
        <span className="ml-auto rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
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

/** Displays a single historical run with full step-by-step timeline. */
export function RunCard({ run }: RunCardProps): ReactNode {
  const allStages = run.iterations.flatMap((iter) => iter.stages);

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt — right-aligned bubble */}
      <div className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
          {run.prompt}
        </div>
      </div>

      {/* Prompt received */}
      <PromptReceivedCard prompt={run.prompt} />

      {/* Iteration stages */}
      {run.iterations.map((iter) => (
        <div key={iter.number} className="flex flex-col gap-2">
          {iter.stages.map((entry) => (
            <div key={entry.id} className="flex flex-col gap-2">
              <HistoryStageCard entry={entry} />

              {/* After prompt_enhance — show enhanced prompt comparison */}
              {entry.stage === "prompt_enhance" && entry.status === "completed" && iter.enhancedPrompt && (
                <PromptCard originalPrompt={run.prompt} enhancedPrompt={iter.enhancedPrompt} durationMs={entry.durationMs} />
              )}

              {/* After plan_audit — show final plan (plan + audit combined) */}
              {entry.stage === "plan_audit" && entry.status === "completed" && (iter.plannerPlan || iter.auditedPlan) && (
                <FinalPlanCard
                  plannerPlan={iter.plannerPlan}
                  auditedPlan={iter.auditedPlan}
                  durationMs={
                    iter.stages
                      .filter((s) => s.stage === "plan" || s.stage === "plan_audit")
                      .reduce((sum, s) => sum + s.durationMs, 0)
                  }
                />
              )}
            </div>
          ))}
        </div>
      ))}

      {/* Result summary — shared component, identical to ChatView */}
      <ResultCard
        status={run.status}
        finalVerdict={run.finalVerdict}
        iterationCount={run.iterations.length}
        totalDurationMs={computeDuration(run.startedAt, run.completedAt)}
        completedAt={run.completedAt}
        executiveSummary={run.executiveSummary}
        error={run.error}
        stageRows={buildStageRows(allStages)}
      />
    </div>
  );
}
