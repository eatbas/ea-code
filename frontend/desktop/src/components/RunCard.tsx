import type { ReactNode } from "react";
import type { AppSettings, RunDetail, PipelineStage, StageResult, StageStatus } from "../types";
import { extractPlanOnly, parseUtcTimestamp } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { StageInputOutputCard } from "./shared/StageInputOutputCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { StageCard } from "./shared/StageCard";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";

interface RunCardProps {
  run: RunDetail;
  settings: AppSettings | null;
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

/** Displays a single historical run with full step-by-step timeline. */
export function RunCard({ run, settings }: RunCardProps): ReactNode {
  const allStages = run.iterations.flatMap((iter) => iter.stages);
  const isTerminalStatus = run.status === "completed" || run.status === "failed" || run.status === "cancelled";
  const isActiveStatus = run.status === "running" || run.status === "waiting_for_input";
  const activeStage = run.currentStage ?? (run.status === "running" ? "prompt_enhance" : undefined);

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt - right-aligned bubble */}
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
          {iter.stages.map((entry) => {
            const stageResult = toStageResult(entry.stage, entry.status, entry.output, entry.durationMs, entry.error);
            const plannerPlan = extractPlanOnly(iter.plannerPlan ?? entry.output);
            const auditedPlan = extractPlanOnly(iter.auditedPlan ?? entry.output);
            const plannerInputForAudit = extractPlanOnly(iter.plannerPlan ?? "");
            const promptEnhanceOutput = (iter.enhancedPrompt ?? entry.output).trim();
            const showPromptEnhanceCard = entry.stage === "prompt_enhance" && entry.status === "completed";
            const showPlanningCard = entry.stage === "plan" && entry.status === "completed";
            const showAuditCard = entry.stage === "plan_audit" && entry.status === "completed";

            return (
              <div key={entry.id} className="flex flex-col gap-2">
                {showPromptEnhanceCard ? (
                  <StageInputOutputCard
                    title="Enhancing Prompt"
                    inputSections={[
                      { label: "Original Prompt", content: run.prompt },
                    ]}
                    outputLabel="Result"
                    outputContent={promptEnhanceOutput || "No valid enhanced prompt output generated."}
                    modelLabel={stageModelLabel("prompt_enhance", settings)}
                    durationMs={entry.durationMs}
                    badgeClassName="bg-emerald-400/25"
                    outputClassName="border border-emerald-400/20 bg-emerald-400/5 text-[#e4e4ed]"
                  />
                ) : showPlanningCard ? (
                  <StageInputOutputCard
                    title="Planning"
                    inputSections={[
                      { label: "Original Prompt", content: run.prompt },
                    ]}
                    outputLabel="Plan"
                    outputContent={plannerPlan || "No valid plan output generated."}
                    modelLabel={stageModelLabel("plan", settings)}
                    durationMs={entry.durationMs}
                    badgeClassName="bg-sky-400/25"
                  />
                ) : showAuditCard ? (
                  <StageInputOutputCard
                    title="Auditing Plan"
                    inputSections={[
                      { label: "Original Prompt", content: run.prompt },
                      { label: "Plan", content: plannerInputForAudit },
                    ]}
                    outputLabel="Audited Plan"
                    outputContent={auditedPlan || "No valid audited plan output generated."}
                    modelLabel={stageModelLabel("plan_audit", settings)}
                    durationMs={entry.durationMs}
                    badgeClassName="bg-amber-400/25"
                    outputClassName="border border-amber-400/20 bg-amber-400/5 text-[#e4e4ed]"
                  />
                ) : (
                  <StageCard stage={stageResult} />
                )}
              </div>
            );
          })}
        </div>
      ))}

      {/* Currently running stage - stage badge plus animated timer */}
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

      {/* Result summary - only shown once the pipeline reaches a terminal state */}
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
