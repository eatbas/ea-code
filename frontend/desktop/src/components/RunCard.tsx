import type { ReactNode } from "react";
import type { AppSettings, RunDetail, PipelineStage, StageResult, StageStatus } from "../types";
import { parseUtcTimestamp, resolveAuditedPlanText, resolvePlanText } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { isActiveStatusValue, isTerminalStatusValue } from "../utils/statusHelpers";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { StageCard } from "./shared/StageCard";
import { RichStageCard } from "./shared/RichStageCard";
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

/** Finds the output of a completed stage by name within the iteration's stages. */
function findStageOutput(stages: RunDetail["iterations"][0]["stages"], stageName: string): string | undefined {
  const found = stages.find((s) => s.stage === stageName && s.status === "completed");
  return found?.output;
}

/** Displays a single historical run with full step-by-step timeline. */
export function RunCard({ run, settings }: RunCardProps): ReactNode {
  const allStages = run.iterations.flatMap((iter) => iter.stages);
  const isTerminalStatus = isTerminalStatusValue(run.status);
  const isActiveStatus = isActiveStatusValue(run.status);
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
          {iter.stages.filter((entry) => entry.stage !== "diff_after_coder" && entry.stage !== "diff_after_code_fixer").map((entry) => {
            const stageResult = toStageResult(entry.stage, entry.status, entry.output, entry.durationMs, entry.error);
            const enhancedPromptText = findStageOutput(iter.stages, "prompt_enhance");
            const planText = findStageOutput(iter.stages, "plan");
            const auditText = findStageOutput(iter.stages, "plan_audit");
            const reviewText = findStageOutput(iter.stages, "code_reviewer");
            const plannerPlan = resolvePlanText(planText, entry.output);
            const auditedPlan = resolveAuditedPlanText(auditText, entry.output);
            const plannerInputForAudit = resolvePlanText(planText);
            const promptEnhanceOutput = (enhancedPromptText ?? entry.output).trim();
            const enhancedPromptInput = (enhancedPromptText ?? run.prompt).trim();

            return (
              <div key={entry.id} className="flex flex-col gap-2">
                <RichStageCard
                  stage={stageResult}
                  runPrompt={run.prompt}
                  enhancedPromptInput={enhancedPromptInput}
                  promptEnhanceOutput={promptEnhanceOutput}
                  planOutput={plannerPlan}
                  planInputForAudit={plannerInputForAudit}
                  auditedPlanOutput={auditedPlan}
                  reviewOutput={reviewText ?? entry.output ?? ""}
                  settings={settings}
                  startedAt={
                    isActiveStatus && run.currentStage === entry.stage
                      ? run.currentStageStartedAt
                        ? parseUtcTimestamp(run.currentStageStartedAt).getTime()
                        : parseUtcTimestamp(run.startedAt).getTime()
                      : undefined
                  }
                />
              </div>
            );
          })}
        </div>
      ))}

      {/* Currently running stage - stage badge plus animated timer */}
      {isActiveStatus && activeStage && (
        <>
          <StageCard
            stage={toStageResult(activeStage, run.status === "waiting_for_input" ? "waiting_for_input" : "running", "", 0)}
            modelLabel={stageModelLabel(activeStage as PipelineStage, settings)}
            startedAt={run.status === "running"
              ? (run.currentStageStartedAt
                ? parseUtcTimestamp(run.currentStageStartedAt).getTime()
                : parseUtcTimestamp(run.startedAt).getTime())
              : undefined}
          />
          {run.status === "running" && activeStage !== "plan_audit" && (
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
