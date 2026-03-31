import type { ReactNode } from "react";
import { useState } from "react";
import type { PipelineStageState } from "../../hooks/usePipelineSession";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";
import { PipelineStageGroup } from "./PipelineStageGroup";
import { PipelineStageSection } from "./PipelineStageSection";
import { PipelineStatusBar } from "./PipelineStatusBar";

interface PipelineConversationViewProps {
  /** User's original prompt. */
  userPrompt: string;
  /** Live stage states from usePipelineSession. */
  stages: PipelineStageState[];
  /** Whether the pipeline is actively running. */
  running: boolean;
  /** Name of the current active stage. */
  currentStageName: string;
  /** Epoch ms when the pipeline started. */
  pipelineStartedAt: number | undefined;
  /** Called when the user clicks Resume / Retry. */
  onResume?: () => void;
  /** Called when the user clicks Stop. */
  onStop?: () => void;
  /** Plan review phase. */
  planReviewPhase?: PlanReviewPhase;
}

/** Return the grid class for a set of parallel stage cards. */
function stageGridClass(count: number): string {
  if (count <= 1) return "";
  if (count === 2) return "grid grid-cols-2 gap-3 min-w-0";
  if (count === 3) return "grid grid-cols-3 gap-3 min-w-0";
  if (count === 4) return "grid grid-cols-2 gap-3 min-w-0";
  // 5+: three columns (3+2, 3+3, 3+3+1, …)
  return "grid grid-cols-3 gap-3 min-w-0";
}

/** Default text shown inside a stage section when no output is available. */
function stageBodyText(stage: PipelineStageState, fallback: string): string {
  if (stage.text) return stage.text;
  if (stage.status === "failed") return "This stage did not produce output.";
  if (stage.status === "completed") return fallback;
  return "Waiting for output...";
}

function isTerminal(status: string): boolean {
  return status === "completed" || status === "failed" || status === "stopped";
}

export function PipelineConversationView({
  userPrompt,
  stages,
  running,
  currentStageName,
  pipelineStartedAt,
  onResume,
  onStop,
  planReviewPhase,
}: PipelineConversationViewProps): ReactNode {
  const [plannersOpen, setPlannersOpen] = useState(true);
  const [mergeOpen, setMergeOpen] = useState(true);
  const [coderOpen, setCoderOpen] = useState(true);
  const [reviewersOpen, setReviewersOpen] = useState(true);
  const [reviewMergeOpen, setReviewMergeOpen] = useState(true);
  const [codeFixerOpen, setCodeFixerOpen] = useState(true);

  // Classify stages by name.
  const plannerStages = stages.filter((s) => s.stageName.startsWith("Planner"));
  const mergeStage = stages.find((s) => s.stageName === "Plan Merge") ?? null;
  const coderStage = stages.find((s) => s.stageName === "Coder") ?? null;
  const reviewerStages = stages.filter((s) => s.stageName.startsWith("Reviewer"));
  const reviewMergeStage = stages.find((s) => s.stageName === "Review Merge") ?? null;
  const codeFixerStage = stages.find((s) => s.stageName === "Code Fixer") ?? null;

  const hasStages = plannerStages.length > 0;
  const allPlannersDone = hasStages && plannerStages.every((s) => isTerminal(s.status));
  const hasFailed = stages.some((s) => s.status === "failed");
  const hasStopped = stages.some((s) => s.status === "stopped");
  const allDone = stages.length > 0 && stages.every((s) => isTerminal(s.status));
  const canResume = allDone && !running
    && planReviewPhase !== "reviewing"
    && planReviewPhase !== "editing"
    && planReviewPhase !== "submitting_edit";
  const planAccepted = planReviewPhase === "accepted";
  const coderDone = coderStage != null && isTerminal(coderStage.status);
  const allReviewersDone = reviewerStages.length > 0 && reviewerStages.every((s) => isTerminal(s.status));
  const reviewMergeDone = reviewMergeStage != null && isTerminal(reviewMergeStage.status);

  const statusBarLabel = currentStageName || (running
    ? "Starting..."
    : hasStopped
      ? "Stopped"
      : stages.length > 0
        ? "Finished"
        : "Starting...");

  return (
    <>
      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden px-5 py-5 pipeline-scroll">
        <div className="mx-auto flex w-full max-w-4xl flex-col gap-3">
          {/* User prompt */}
          <div className="ml-auto max-w-3xl rounded-2xl border border-edge-strong bg-elevated px-4 py-3 text-sm leading-6 text-fg">
            <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
              user
            </p>
            <p className="whitespace-pre-wrap break-words">{userPrompt}</p>
          </div>

          {/* Planner stages (parallel — synced open/close) */}
          {hasStages && (
            <PipelineStageGroup groupLabel="Planners" stages={plannerStages}>
              <div className={stageGridClass(plannerStages.length)}>
                {plannerStages.map((stage, i) => (
                  <PipelineStageSection
                    key={`planner-${String(i)}`}
                    label={stage.stageName || `Planner ${String(i + 1)}`}
                    agentLabel={stage.agentLabel}
                    status={stage.status}
                    open={plannersOpen}
                    onOpenChange={setPlannersOpen}
                    startedAt={stage.startedAt}
                    finishedAt={stage.finishedAt}
                  >
                    <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                      {stageBodyText(stage, "Plan file was not found.")}
                    </p>
                  </PipelineStageSection>
                ))}
              </div>
            </PipelineStageGroup>
          )}

          {/* Plan Merge stage */}
          {mergeStage ? (
            <PipelineStageSection
              label="Plan Merge"
              agentLabel={mergeStage.agentLabel}
              status={mergeStage.status}
              open={mergeOpen}
              onOpenChange={setMergeOpen}
              startedAt={mergeStage.startedAt}
              finishedAt={mergeStage.finishedAt}
            >
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                {stageBodyText(mergeStage, "Merged plan file was not found.")}
              </p>
            </PipelineStageSection>
          ) : (
            allPlannersDone && !running && (
              <PipelineStageSection label="Plan Merge" status="pending">
                <p className="text-xs text-fg-faint">Waiting for planners to finish...</p>
              </PipelineStageSection>
            )
          )}

          {/* Coder stage */}
          {coderStage ? (
            <PipelineStageSection
              label="Coder"
              agentLabel={coderStage.agentLabel}
              status={coderStage.status}
              open={coderOpen}
              onOpenChange={setCoderOpen}
              startedAt={coderStage.startedAt}
              finishedAt={coderStage.finishedAt}
            >
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                {stageBodyText(coderStage, "Coder completion summary was not found.")}
              </p>
            </PipelineStageSection>
          ) : (
            planAccepted && (
              <PipelineStageSection label="Coder" status="pending">
                <p className="text-xs text-fg-faint">Plan accepted. Coder stage pending...</p>
              </PipelineStageSection>
            )
          )}

          {/* Reviewer stages (parallel — synced open/close) */}
          {reviewerStages.length > 0 && (
            <PipelineStageGroup groupLabel="Reviewers" stages={reviewerStages}>
              <div className={stageGridClass(reviewerStages.length)}>
                {reviewerStages.map((stage, i) => (
                  <PipelineStageSection
                    key={`reviewer-${String(i)}`}
                    label={stage.stageName || `Reviewer ${String(i + 1)}`}
                    agentLabel={stage.agentLabel}
                    status={stage.status}
                    open={reviewersOpen}
                    onOpenChange={setReviewersOpen}
                    startedAt={stage.startedAt}
                    finishedAt={stage.finishedAt}
                  >
                    <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                      {stageBodyText(stage, "Review file was not found.")}
                    </p>
                  </PipelineStageSection>
                ))}
              </div>
            </PipelineStageGroup>
          )}
          {reviewerStages.length === 0 && coderDone && running && (
            <PipelineStageSection label="Reviewers" status="pending">
              <p className="text-xs text-fg-faint">Coder complete. Reviewer stages pending...</p>
            </PipelineStageSection>
          )}

          {/* Review Merge stage */}
          {reviewMergeStage ? (
            <PipelineStageSection
              label="Review Merge"
              agentLabel={reviewMergeStage.agentLabel}
              status={reviewMergeStage.status}
              open={reviewMergeOpen}
              onOpenChange={setReviewMergeOpen}
              startedAt={reviewMergeStage.startedAt}
              finishedAt={reviewMergeStage.finishedAt}
            >
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                {stageBodyText(reviewMergeStage, "Merged review file was not found.")}
              </p>
            </PipelineStageSection>
          ) : (
            allReviewersDone && running && (
              <PipelineStageSection label="Review Merge" status="pending">
                <p className="text-xs text-fg-faint">Reviews complete. Merging reviews...</p>
              </PipelineStageSection>
            )
          )}

          {/* Code Fixer stage */}
          {codeFixerStage ? (
            <PipelineStageSection
              label="Code Fixer"
              agentLabel={codeFixerStage.agentLabel}
              status={codeFixerStage.status}
              open={codeFixerOpen}
              onOpenChange={setCodeFixerOpen}
              startedAt={codeFixerStage.startedAt}
              finishedAt={codeFixerStage.finishedAt}
            >
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                {stageBodyText(codeFixerStage, "Code Fixer summary was not found.")}
              </p>
            </PipelineStageSection>
          ) : (
            reviewMergeDone && running && (
              <PipelineStageSection label="Code Fixer" status="pending">
                <p className="text-xs text-fg-faint">Review merge complete. Code Fixer pending...</p>
              </PipelineStageSection>
            )
          )}

          {!hasStages && (
            <div className="flex items-center justify-center py-10">
              <p className="text-sm text-fg-faint">Starting pipeline...</p>
            </div>
          )}

        </div>
      </div>

      {/* Status bar — always visible at the bottom with Stop/Resume/Review */}
      {pipelineStartedAt && (
        <PipelineStatusBar
          stageName={statusBarLabel}
          running={running}
          startedAt={pipelineStartedAt}
          canResume={canResume}
          hasFailed={hasFailed}
          onResume={onResume}
          onStop={onStop}
          reviewPhase={planReviewPhase}
        />
      )}
    </>
  );
}
