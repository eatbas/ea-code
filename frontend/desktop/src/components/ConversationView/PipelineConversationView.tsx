import type { ReactNode } from "react";
import { useState } from "react";
import { RotateCcw, Square } from "lucide-react";
import type { PipelineStageState } from "../../hooks/usePipelineSession";
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
}

export function PipelineConversationView({
  userPrompt,
  stages,
  running,
  currentStageName,
  pipelineStartedAt,
  onResume,
  onStop,
}: PipelineConversationViewProps): ReactNode {
  const [plannersOpen, setPlannersOpen] = useState(true);
  const [mergeOpen, setMergeOpen] = useState(true);

  // Separate planner stages from the merge stage.
  const plannerStages = stages.filter((s) => s.stageName !== "Plan Merge");
  const mergeStage = stages.find((s) => s.stageName === "Plan Merge") ?? null;

  const hasStages = plannerStages.length > 0;
  const allPlannersDone = hasStages && plannerStages.every((s) => (
    s.status === "completed" || s.status === "failed" || s.status === "stopped"
  ));
  const hasFailed = stages.some((s) => s.status === "failed");
  const hasStopped = stages.some((s) => s.status === "stopped");
  const allDone = stages.length > 0 && stages.every((s) => (
    s.status === "completed" || s.status === "failed" || s.status === "stopped"
  ));
  const canResume = allDone && !running;
  const mergeCompleted = mergeStage?.status === "completed";
  const statusBarLabel = currentStageName || (running
    ? "Starting..."
    : hasStopped
      ? "Stopped"
      : stages.length > 0
        ? "Finished"
        : "Starting...");

  return (
    <>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5 pipeline-scroll">
        <div className="mx-auto flex w-full max-w-4xl flex-col gap-3">
          {/* User prompt */}
          <div className="ml-auto max-w-3xl rounded-2xl border border-edge-strong bg-elevated px-4 py-3 text-sm leading-6 text-fg">
            <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
              user
            </p>
            <p className="whitespace-pre-wrap">{userPrompt}</p>
          </div>

          {/* Planner stages (parallel — synced open/close) */}
          {hasStages && (
            <div className={plannerStages.length > 1 ? "grid grid-cols-2 gap-3" : ""}>
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
                  <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap">
                    {stage.text || (stage.status === "failed"
                      ? "This stage did not produce output."
                      : stage.status === "completed"
                      ? "Plan file was not found."
                      : "Waiting for output...")}
                  </p>
                </PipelineStageSection>
              ))}
            </div>
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
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap">
                {mergeStage.text || (mergeStage.status === "failed"
                  ? "This stage did not produce output."
                  : mergeStage.status === "completed"
                  ? "Merged plan file was not found."
                  : "Merging plans...")}
              </p>
            </PipelineStageSection>
          ) : (
            allPlannersDone && !running && (
              <PipelineStageSection label="Plan Merge" status="pending">
                <p className="text-xs text-fg-faint">Waiting for planners to finish...</p>
              </PipelineStageSection>
            )
          )}

          {/* Coder placeholder — only after merge completes */}
          {mergeCompleted && (
            <PipelineStageSection label="Coder" status="pending">
              <p className="text-xs text-fg-faint">Waiting for merged plan...</p>
            </PipelineStageSection>
          )}

          {!hasStages && (
            <div className="flex items-center justify-center py-10">
              <p className="text-sm text-fg-faint">Starting pipeline...</p>
            </div>
          )}

          {/* Resume / Stop controls */}
          {canResume && onResume && (
            <div className="flex items-center justify-center gap-3 py-2">
              <button
                type="button"
                onClick={onResume}
                className="inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-4 py-2 text-xs font-semibold text-fg transition-colors hover:bg-active"
              >
                <RotateCcw size={12} />
                {hasFailed ? "Retry Pipeline" : "Resume Pipeline"}
              </button>
            </div>
          )}
          {running && onStop && (
            <div className="flex items-center justify-center gap-3 py-2">
              <button
                type="button"
                onClick={onStop}
                className="inline-flex items-center gap-2 rounded-lg border border-error-border bg-error-bg px-4 py-2 text-xs font-semibold text-error-text transition-colors hover:opacity-80"
              >
                <Square size={10} fill="currentColor" />
                Stop Pipeline
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Status bar */}
      {pipelineStartedAt && (
        <PipelineStatusBar
          stageName={statusBarLabel}
          running={running}
          startedAt={pipelineStartedAt}
        />
      )}
    </>
  );
}
