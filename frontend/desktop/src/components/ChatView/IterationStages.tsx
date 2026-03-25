import type { ReactNode } from "react";
import type { PipelineRun, PipelineStage, AppSettings, StageResult } from "../../types";
import { isActive } from "../../utils/statusHelpers";
import { resolveAuditedPlanText, resolvePlanText } from "../../utils/formatters";
import { stageModelLabel } from "../../utils/stageModelLabels";
import { StageInputOutputCard } from "../shared/StageInputOutputCard";
import { RichStageCard } from "../shared/RichStageCard";
import { TabbedPlanCard, isPlanStage } from "../shared/TabbedPlanCard";
import { TabbedReviewCard, isReviewStage } from "../shared/TabbedReviewCard";
import { PlannerProgressRow } from "../shared/PlannerProgressRow";
import { ReviewerProgressRow } from "../shared/ReviewerProgressRow";

/** Pre-computed data for a single iteration. */
export interface IterationGroup {
  iterNum: number;
  stages: StageResult[];
  planStages: StageResult[];
  reviewStages: StageResult[];
  isLatest: boolean;
}

interface IterationStagesProps {
  group: IterationGroup;
  run: PipelineRun;
  artifacts: Record<string, string>;
  planArtifactMap: Record<string, string>;
  reviewArtifactMap: Record<string, string>;
  enhancedPromptInput: string;
  planArtifact: string | undefined;
  planAuditArtifact: string | undefined;
  planInputForAudit: string;
  settings: AppSettings | null;
  isPaused: boolean;
}

/** Renders the stages of a single iteration with per-iteration plan/review grouping. */
export function IterationStages({
  group,
  run,
  artifacts,
  planArtifactMap,
  reviewArtifactMap,
  enhancedPromptInput,
  planArtifact,
  planAuditArtifact,
  planInputForAudit,
  settings,
  isPaused,
}: IterationStagesProps): ReactNode {
  const { stages, planStages, reviewStages, isLatest, iterNum } = group;

  // For the latest iteration, use live artifacts. For past iterations, use stage output.
  const iterPlanAuditArtifact = isLatest ? planAuditArtifact : undefined;
  const iterPlanArtifact = isLatest ? planArtifact : undefined;
  const iterPlanInputForAudit = isLatest ? planInputForAudit : "";

  // Find the last completed plan_audit within THIS iteration.
  const lastCompletedPlanAuditIdx = stages.reduce(
    (latest, stage, idx) => (stage.stage === "plan_audit" && stage.status === "completed" ? idx : latest), -1,
  );

  let planGroupRendered = false;
  let reviewGroupRendered = false;

  return (
    <>
      {stages.map((stage, stageIdx) => {
        // Group plan stages into a single tabbed card per iteration.
        if (isPlanStage(stage.stage)) {
          if (planGroupRendered) return null;
          planGroupRendered = true;
          const planningActive = isActive(run.status) && isLatest && isPlanStage(run.currentStage as PipelineStage);
          return (
            <div key={`plan-group-${iterNum}`} className="flex flex-col gap-2">
              <TabbedPlanCard
                planStages={planStages}
                planArtifacts={planArtifactMap}
                runPrompt={run.prompt}
                enhancedPromptInput={enhancedPromptInput}
                settings={settings}
                startedAt={planningActive ? run.stageStartedAt : undefined}
                runStatus={run.status}
              />
              {(planningActive || planStages.some((s) => s.status === "running")) && (
                <PlannerProgressRow
                  stages={planStages}
                  settings={settings}
                  isPaused={isPaused}
                />
              )}
            </div>
          );
        }

        // Group review stages into a single tabbed card per iteration.
        if (isReviewStage(stage.stage)) {
          if (reviewGroupRendered) return null;
          reviewGroupRendered = true;
          const reviewingActive = isActive(run.status) && isLatest && isReviewStage(run.currentStage as PipelineStage);
          return (
            <div key={`review-group-${iterNum}`} className="flex flex-col gap-2">
              <TabbedReviewCard
                reviewStages={reviewStages}
                reviewArtifacts={reviewArtifactMap}
                runPrompt={run.prompt}
                enhancedPromptInput={enhancedPromptInput}
                settings={settings}
                startedAt={reviewingActive ? run.stageStartedAt : undefined}
                runStatus={run.status}
              />
              {(reviewingActive || reviewStages.some((s) => s.status === "running")) && (
                <ReviewerProgressRow
                  stages={reviewStages}
                  settings={settings}
                  isPaused={isPaused}
                />
              )}
            </div>
          );
        }

        return (
          <div key={`${stage.stage}-${iterNum}-${stageIdx}`} className="flex flex-col gap-2">
            {stage.stage === "judge" && stage.status === "completed" ? (
              <StageInputOutputCard
                title="Judge"
                inputSections={[
                  { label: "Original Prompt", content: run.prompt },
                  { label: "Enhanced Prompt", content: enhancedPromptInput },
                  { label: "Plan", content: resolveAuditedPlanText(iterPlanAuditArtifact, iterPlanArtifact) },
                  { label: "Review Findings", content: [...stages.slice(0, stageIdx)].reverse().find((entry) => entry.stage === "code_reviewer")?.output ?? artifacts["review"] ?? "" },
                  { label: "Fixer Output", content: [...stages.slice(0, stageIdx)].reverse().find((entry) => entry.stage === "code_fixer")?.output ?? "" },
                ]}
                outputLabel="Decision"
                outputContent={isLatest ? (artifacts["judge"] ?? stage.output ?? "No judge output generated.") : (stage.output || "No judge output generated.")}
                modelLabel={stageModelLabel("judge", settings)}
                durationMs={stage.durationMs}
                badgeClassName="bg-rose-400/25"
                outputClassName="border border-rose-400/20 bg-rose-400/5 text-[#e4e4ed]"
              />
            ) : (
              <RichStageCard
                stage={stage}
                runPrompt={run.prompt}
                enhancedPromptInput={enhancedPromptInput}
                promptEnhanceOutput={(isLatest ? (artifacts["enhanced_prompt"] ?? stage.output) : stage.output).trim()}
                planOutput={resolvePlanText(iterPlanArtifact, stage.output)}
                planInputForAudit={iterPlanInputForAudit}
                auditedPlanOutput={resolveAuditedPlanText(iterPlanAuditArtifact, stage.output)}
                settings={settings}
                startedAt={
                  run.status === "running" && isLatest && run.currentStage === stage.stage && stage.status === "running"
                    ? run.stageStartedAt
                    : undefined
                }
                showPlanCard={false}
                showPlanAuditCard={stageIdx === lastCompletedPlanAuditIdx}
              />
            )}
          </div>
        );
      })}
    </>
  );
}
