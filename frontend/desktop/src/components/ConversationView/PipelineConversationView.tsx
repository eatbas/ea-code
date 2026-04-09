import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { ChevronDown, Clipboard } from "lucide-react";
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
  /** Persistent backend debug trace for this pipeline conversation. */
  debugLog: string;
  /** Whether the pipeline is actively running. */
  running: boolean;
  /** Name of the current active stage. */
  currentStageName: string;
  /** Epoch ms when the pipeline started. */
  pipelineStartedAt: number | undefined;
  /** Called when the user clicks Resume / Retry. */
  onResume?: () => void;
  /** Called when the user clicks Re-do Review. */
  onRedoReview?: () => void;
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
  debugLog,
  running,
  currentStageName,
  pipelineStartedAt,
  onResume,
  onRedoReview,
  onStop,
  planReviewPhase,
}: PipelineConversationViewProps): ReactNode {
  const [orchestratorOpen, setOrchestratorOpen] = useState(true);
  const [plannersOpen, setPlannersOpen] = useState(true);
  const [mergeOpen, setMergeOpen] = useState(true);
  const [coderOpen, setCoderOpen] = useState(true);
  const [reviewersOpen, setReviewersOpen] = useState(true);
  const [reviewMergeOpen, setReviewMergeOpen] = useState(true);
  const [codeFixerOpen, setCodeFixerOpen] = useState(true);
  const [debugOpen, setDebugOpen] = useState(true);
  const [copiedDebug, setCopiedDebug] = useState(false);

  // Classify stages by name.
  const orchestratorStage = stages.find((s) => s.stageName === "Prompt Enhancer") ?? null;
  const plannerStages = stages.filter((s) => s.stageName.startsWith("Planner"));
  const mergeStage = stages.find((s) => s.stageName === "Plan Merge") ?? null;
  const coderStage = stages.find((s) => s.stageName === "Coder") ?? null;

  // Separate first-run review stages from re-do cycle stages.
  const reviewerStages = stages.filter(
    (s) => s.stageName.startsWith("Reviewer") && !s.stageName.includes("(Cycle"),
  );
  const reviewMergeStage = stages.find(
    (s) => s.stageName === "Review Merge",
  ) ?? null;
  const codeFixerStage = stages.find(
    (s) => s.stageName === "Code Fixer",
  ) ?? null;

  // Collect re-do review cycles. Each cycle has stages with "(Cycle N)" suffix.
  const redoCycles: Array<{
    cycle: number;
    reviewers: PipelineStageState[];
    reviewMerge: PipelineStageState | null;
    codeFixer: PipelineStageState | null;
  }> = [];
  const cycleNumbers = new Set<number>();
  for (const s of stages) {
    const match = s.stageName.match(/\(Cycle (\d+)\)/);
    if (match) cycleNumbers.add(Number(match[1]));
  }
  for (const cycle of [...cycleNumbers].sort((a, b) => a - b)) {
    const suffix = `(Cycle ${String(cycle)})`;
    redoCycles.push({
      cycle,
      reviewers: stages.filter(
        (s) => s.stageName.startsWith("Reviewer") && s.stageName.includes(suffix),
      ),
      reviewMerge: stages.find(
        (s) => s.stageName.includes("Review Merge") && s.stageName.includes(suffix),
      ) ?? null,
      codeFixer: stages.find(
        (s) => s.stageName.includes("Code Fixer") && s.stageName.includes(suffix),
      ) ?? null,
    });
  }

  const hasStages = plannerStages.length > 0;
  const allPlannersDone = hasStages && plannerStages.every((s) => isTerminal(s.status));
  const hasFailed = stages.some((s) => s.status === "failed");
  const hasStopped = stages.some((s) => s.status === "stopped");
  const allDone = stages.length > 0 && stages.every((s) => isTerminal(s.status));

  // Derive the pipeline finish timestamp from the latest stage finishedAt so
  // the total timer stops counting once the pipeline is no longer running.
  const pipelineFinishedAt = !running && allDone
    ? stages.reduce<number | undefined>((latest, s) => {
      if (s.finishedAt === undefined) return latest;
      return latest === undefined ? s.finishedAt : Math.max(latest, s.finishedAt);
    }, undefined)
    : undefined;
  const canResume = allDone && !running
    && planReviewPhase !== "reviewing"
    && planReviewPhase !== "editing"
    && planReviewPhase !== "submitting_edit";
  const planAccepted = planReviewPhase === "accepted";
  const coderDone = coderStage != null && isTerminal(coderStage.status);
  const allReviewersDone = reviewerStages.length > 0 && reviewerStages.every((s) => isTerminal(s.status));
  const reviewMergeDone = reviewMergeStage != null && isTerminal(reviewMergeStage.status);

  // The latest code fixer is the one from the last redo cycle, or the original.
  const latestCodeFixer = redoCycles.length > 0
    ? redoCycles[redoCycles.length - 1].codeFixer
    : codeFixerStage;
  const canRedoReview = allDone && !running && latestCodeFixer != null
    && isTerminal(latestCodeFixer.status)
    && planReviewPhase !== "reviewing"
    && planReviewPhase !== "editing"
    && planReviewPhase !== "submitting_edit";

  // Auto-collapse stages when they complete.
  useEffect(() => {
    if (orchestratorStage && isTerminal(orchestratorStage.status)) setOrchestratorOpen(false);
    // Depend only on the status string, not the full object reference, so
    // the user can re-open the section without it being forced closed on
    // every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [orchestratorStage?.status]);

  useEffect(() => {
    if (allPlannersDone) setPlannersOpen(false);
  }, [allPlannersDone]);

  useEffect(() => {
    if (mergeStage && isTerminal(mergeStage.status)) setMergeOpen(false);
  }, [mergeStage?.status]);

  useEffect(() => {
    if (coderStage && isTerminal(coderStage.status)) setCoderOpen(false);
  }, [coderStage?.status]);

  useEffect(() => {
    if (allReviewersDone) setReviewersOpen(false);
  }, [allReviewersDone]);

  useEffect(() => {
    if (reviewMergeStage && isTerminal(reviewMergeStage.status)) setReviewMergeOpen(false);
  }, [reviewMergeStage?.status]);

  useEffect(() => {
    if (codeFixerStage && isTerminal(codeFixerStage.status)) setCodeFixerOpen(false);
  }, [codeFixerStage?.status]);

  useEffect(() => {
    if (!copiedDebug) return;
    const id = setTimeout(() => setCopiedDebug(false), 1500);
    return () => clearTimeout(id);
  }, [copiedDebug]);

  async function handleCopyDebugLog(): Promise<void> {
    if (!debugLog.trim()) return;
    await navigator.clipboard.writeText(debugLog);
    setCopiedDebug(true);
  }

  // The status bar label comes directly from the backend stage name now.
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

          {/* Prompt Enhancer stage */}
          {orchestratorStage ? (
            <PipelineStageSection
              label="Prompt Enhancer"
              agentLabel={orchestratorStage.agentLabel}
              status={orchestratorStage.status}
              open={orchestratorOpen}
              onOpenChange={setOrchestratorOpen}
              startedAt={orchestratorStage.startedAt}
              finishedAt={orchestratorStage.finishedAt}
            >
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                {stageBodyText(orchestratorStage, "Enhanced prompt was not found.")}
              </p>
            </PipelineStageSection>
          ) : (
            stages.length === 0 && running && (
              <PipelineStageSection label="Prompt Enhancer" status="pending">
                <p className="text-xs text-fg-faint">Enhancing prompt...</p>
              </PipelineStageSection>
            )
          )}

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

          {/* Re-do review cycles */}
          {redoCycles.map((cycle) => (
            <div key={`cycle-${String(cycle.cycle)}`} className="flex flex-col gap-3">
              <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle mt-2">
                Review Cycle {cycle.cycle}
              </p>

              {cycle.reviewers.length > 0 && (
                <PipelineStageGroup groupLabel={`Reviewers (Cycle ${String(cycle.cycle)})`} stages={cycle.reviewers}>
                  <div className={stageGridClass(cycle.reviewers.length)}>
                    {cycle.reviewers.map((stage, i) => (
                      <PipelineStageSection
                        key={`redo-reviewer-${String(cycle.cycle)}-${String(i)}`}
                        label={stage.stageName}
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

              {cycle.reviewMerge && (
                <PipelineStageSection
                  label={cycle.reviewMerge.stageName}
                  agentLabel={cycle.reviewMerge.agentLabel}
                  status={cycle.reviewMerge.status}
                  open={reviewMergeOpen}
                  onOpenChange={setReviewMergeOpen}
                  startedAt={cycle.reviewMerge.startedAt}
                  finishedAt={cycle.reviewMerge.finishedAt}
                >
                  <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                    {stageBodyText(cycle.reviewMerge, "Merged review file was not found.")}
                  </p>
                </PipelineStageSection>
              )}

              {cycle.codeFixer && (
                <PipelineStageSection
                  label={cycle.codeFixer.stageName}
                  agentLabel={cycle.codeFixer.agentLabel}
                  status={cycle.codeFixer.status}
                  open={codeFixerOpen}
                  onOpenChange={setCodeFixerOpen}
                  startedAt={cycle.codeFixer.startedAt}
                  finishedAt={cycle.codeFixer.finishedAt}
                >
                  <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap break-words">
                    {stageBodyText(cycle.codeFixer, "Code Fixer summary was not found.")}
                  </p>
                </PipelineStageSection>
              )}
            </div>
          ))}

          {import.meta.env.VITE_MAESTRO_DEV === "true" && (
            <div className="rounded-2xl border border-edge bg-panel">
              <div className="flex items-center justify-between border-b border-edge px-4 py-3">
                <button
                  type="button"
                  onClick={() => setDebugOpen((open) => !open)}
                  className="flex min-w-0 flex-1 items-center gap-3 text-left"
                  aria-expanded={debugOpen}
                >
                  <div className="min-w-0 flex-1">
                    <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                      Pipeline Debug
                    </p>
                    <p className="text-xs text-fg-muted">
                      Tauri backend trace. Copy this and send it back for diagnosis.
                    </p>
                  </div>
                  <ChevronDown
                    size={14}
                    className={`shrink-0 text-fg-muted transition-transform ${debugOpen ? "rotate-180" : ""}`}
                  />
                </button>
                <button
                  type="button"
                  onClick={() => { void handleCopyDebugLog(); }}
                  disabled={!debugLog.trim()}
                  className="ml-3 inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-3 py-1.5 text-xs font-semibold text-fg transition-colors hover:bg-active disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <Clipboard size={12} />
                  {copiedDebug ? "Copied" : "Copy Log"}
                </button>
              </div>
              {debugOpen && (
                <div className="max-h-56 overflow-auto px-4 py-3 pipeline-scroll">
                  <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-fg-muted">
                    {debugLog || "Waiting for pipeline debug output..."}
                  </pre>
                </div>
              )}
            </div>
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
          finishedAt={pipelineFinishedAt}
          canResume={canResume}
          hasFailed={hasFailed}
          onResume={onResume}
          canRedoReview={canRedoReview}
          onRedoReview={onRedoReview}
          onStop={onStop}
          reviewPhase={planReviewPhase}
        />
      )}
    </>
  );
}
