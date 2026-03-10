import { useState } from "react";
import type { ReactNode } from "react";
import type { RunDetail, PipelineStage, StageResult, StageStatus } from "../types";
import { parseUtcTimestamp } from "../utils/formatters";
import { PromptCard } from "./shared/PromptCard";
import { FinalPlanCard } from "./shared/FinalPlanCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { StageCard } from "./shared/StageCard";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";

interface RunCardProps {
  run: RunDetail;
}

function toStageResult(stage: string, status: string, output: string, durationMs: number, error?: string): StageResult {
  return {
    stage: stage as PipelineStage,
    status: status as StageStatus,
    output,
    durationMs,
    error,
  };
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
  const isTerminalStatus = run.status === "completed" || run.status === "failed" || run.status === "cancelled";
  const isActiveStatus = run.status === "running" || run.status === "waiting_for_input";
  const activeStage = run.currentStage ?? (run.status === "running" ? "prompt_enhance" : undefined);

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
              <StageCard stage={toStageResult(entry.stage, entry.status, entry.output, entry.durationMs, entry.error)} />

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

      {/* Currently running stage — shows the stage name badge (matching ChatView) + animated timer */}
      {isActiveStatus && activeStage && (
        <>
          <StageCard stage={toStageResult(activeStage, run.status === "waiting_for_input" ? "waiting_for_input" : "running", "", 0)} />
          {run.status === "running" && (
            <ThinkingIndicator
              stage={activeStage as PipelineStage}
              startedAt={run.currentStageStartedAt
                ? parseUtcTimestamp(run.currentStageStartedAt).getTime()
                : parseUtcTimestamp(run.startedAt).getTime()}
            />
          )}
        </>
      )}

      {/* Result summary — only shown once the pipeline reaches a terminal state */}
      {isTerminalStatus && (
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
      )}
    </div>
  );
}
