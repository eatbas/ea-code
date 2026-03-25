import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, PipelineStage, RunSummary, RunEvent, StageResult, StageStatus } from "../types";
import { parseUtcTimestamp } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { isActive, isTerminal } from "../utils/statusHelpers";
import { buildPlanArtifactMap, buildReviewArtifactMap } from "../utils/artifactHelpers";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { StageCard } from "./shared/StageCard";
import { RichStageCard } from "./shared/RichStageCard";
import { TabbedPlanCard, isPlanStage } from "./shared/TabbedPlanCard";
import { TabbedReviewCard, isReviewStage } from "./shared/TabbedReviewCard";
import { ResultCard, buildStageRowsFromEvents, computeDuration } from "./shared/ResultCard";

interface RunCardProps {
  run: RunSummary;
  settings: AppSettings | null;
  /** Workspace path required for loading run events and artifacts. */
  workspacePath: string;
  /** When true, hides the user prompt bubble (rendered separately in message-driven view). */
  hidePromptBubble?: boolean;
}

/** Converts RunEvent array to StageResult array for display purposes.
 *  Includes both completed stages (stage_end) and in-progress stages
 *  (stage_start without a matching stage_end).
 */
function eventsToStageResults(events: RunEvent[]): StageResult[] {
  const stageMap = new Map<string, {
    stage: string;
    status: StageStatus;
    output: string;
    durationMs: number;
    startedAt?: number;
  }>();

  // First pass: collect stage_start events as "running" placeholders.
  for (const event of events) {
    if (event.type === "stage_start" && event.stage) {
      const key = `${event.stage}-${event.iteration}`;
      const existing = stageMap.get(key);
      stageMap.set(key, {
        stage: event.stage,
        status: existing?.status ?? "running",
        output: existing?.output ?? "",
        durationMs: existing?.durationMs ?? 0,
        startedAt: existing?.startedAt ?? parseUtcTimestamp(event.ts).getTime(),
      });
    }
  }

  // Second pass: overwrite with stage_end data (completed/failed).
  for (const event of events) {
    if (event.type === "stage_end" && event.stage) {
      const key = `${event.stage}-${event.iteration}`;
      const existing = stageMap.get(key);
      stageMap.set(key, {
        stage: event.stage,
        status: event.status as StageStatus,
        output: existing?.output ?? "",
        durationMs: event.durationMs ?? 0,
        startedAt: existing?.startedAt,
      });
    }
  }

  return Array.from(stageMap.values()).map((s) => ({
    stage: s.stage as PipelineStage,
    status: s.status,
    output: s.output,
    durationMs: s.durationMs,
    startedAt: s.startedAt,
  }));
}

/** Displays a single historical run with full step-by-step timeline.
 *  Events are lazy-loaded when the card is expanded.
 */
export function RunCard({ run, settings, workspacePath, hidePromptBubble }: RunCardProps): ReactNode {
  const isTerminalStatus = isTerminal(run.status);
  const isActiveStatus = isActive(run.status);
  const [events, setEvents] = useState<RunEvent[] | null>(null);
  const [artifacts, setArtifacts] = useState<Record<string, string>>({});
  const [loadingEvents, setLoadingEvents] = useState(false);
  const [isExpanded, setIsExpanded] = useState(() => isTerminalStatus || isActiveStatus);

  const loadEvents = useCallback(async (force = false) => {
    if ((!force && events) || loadingEvents) return;
    setLoadingEvents(true);
    try {
      const [runEvents, runArtifacts] = await Promise.all([
        invoke<RunEvent[]>("get_run_events", { runId: run.id, sessionId: run.sessionId, workspacePath }),
        invoke<Record<string, string>>("get_run_artifacts", { runId: run.id, sessionId: run.sessionId, workspacePath }),
      ]);
      setEvents(runEvents);
      setArtifacts(runArtifacts);
    } catch (e) {
      console.error("Failed to load events:", e);
    } finally {
      setLoadingEvents(false);
    }
  }, [events, loadingEvents, run.id, run.sessionId, workspacePath]);

  // Load events when expanded
  useEffect(() => {
    if (isExpanded) {
      void loadEvents();
    }
  }, [isExpanded, loadEvents]);

  // Poll events/artifacts for active runs so new stages appear.
  useEffect(() => {
    if (!isActiveStatus || !isExpanded) return;
    const interval = setInterval(() => { void loadEvents(true); }, 3000);
    return () => clearInterval(interval);
  }, [isActiveStatus, isExpanded, loadEvents]);
  const activeStage = run.currentStage ?? (run.status === "running" ? "prompt_enhance" : undefined);

  // Keep run details visible when reopening a session or when a live run completes.
  useEffect(() => {
    if (isActiveStatus || isTerminalStatus) {
      setIsExpanded(true);
    }
  }, [isActiveStatus, isTerminalStatus]);

  const stageResults = events ? eventsToStageResults(events) : [];

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt - right-aligned bubble (hidden when rendered from message timeline) */}
      {!hidePromptBubble && (
        <div className="flex justify-end">
          <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
            {run.prompt}
          </div>
        </div>
      )}

      {/* Expand/collapse button for terminal runs */}
      {isTerminalStatus && (
        <div className="flex justify-center">
          <button
            onClick={() => setIsExpanded(!isExpanded)}
            disabled={loadingEvents}
            className="flex items-center gap-1 rounded-lg border border-[#2e2e48] bg-[#1a1a24] px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors disabled:opacity-50"
          >
            {loadingEvents ? (
              <>
                <svg className="h-3 w-3 animate-spin" viewBox="0 0 24 24" fill="none">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
                Loading details...
              </>
            ) : (
              <>
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  className={`transition-transform ${isExpanded ? "rotate-180" : ""}`}
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
                {isExpanded ? "Hide details" : "Show details"}
              </>
            )}
          </button>
        </div>
      )}

      {/* Prompt received */}
      <PromptReceivedCard prompt={run.prompt} />

      {/* Expanded content: stages and results */}
      {(isExpanded || isActiveStatus) && (
        <>
          {/* Stage results from events */}
          {stageResults.length > 0 && (
            <RunStageList
              stageResults={stageResults}
              run={run}
              artifacts={artifacts}
              settings={settings}
              isActiveStatus={isActiveStatus}
            />
          )}

          {/* Currently running stage - stage badge plus animated timer */}
          {isActiveStatus && activeStage && (
            <>
              <StageCard
                stage={{
                  stage: activeStage as PipelineStage,
                  status: run.status === "waiting_for_input" ? "waiting_for_input" as const : "running" as const,
                  output: "",
                  durationMs: 0,
                }}
                modelLabel={stageModelLabel(activeStage as PipelineStage, settings)}
                startedAt={run.status === "running"
                  ? (run.startedAt
                    ? parseUtcTimestamp(run.startedAt).getTime()
                    : undefined)
                  : undefined}
              />
              {run.status === "running" && (
                <ThinkingIndicator
                  stage={activeStage as PipelineStage}
                  startedAt={run.startedAt
                    ? parseUtcTimestamp(run.startedAt).getTime()
                    : undefined}
                />
              )}
            </>
          )}

          {/* Result summary - only shown once the pipeline reaches a terminal state */}
          {isTerminalStatus && events && (
            <ResultCard
              status={run.status}
              finalVerdict={run.finalVerdict ?? undefined}
              iterationCount={run.currentIteration || 1}
              totalDurationMs={computeDuration(run.startedAt, run.completedAt ?? undefined)}
              completedAt={run.completedAt ?? undefined}
              executiveSummary={run.executiveSummary ?? artifacts["executive_summary"]}
              error={run.error}
              stageRows={buildStageRowsFromEvents(events)}
              judgeReasoning={artifacts["judge"]}
            />
          )}
        </>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stage list sub-component — groups plan stages into TabbedPlanCard
// ---------------------------------------------------------------------------

interface RunStageListProps {
  stageResults: StageResult[];
  run: RunSummary;
  artifacts: Record<string, string>;
  settings: AppSettings | null;
  isActiveStatus: boolean;
}

function RunStageList({ stageResults, run, artifacts, settings, isActiveStatus }: RunStageListProps) {
  const planGroupStages = stageResults.filter((s) => isPlanStage(s.stage));
  const planArtifactMap = buildPlanArtifactMap(artifacts);

  const reviewGroupStages = stageResults.filter((s) => isReviewStage(s.stage));
  const reviewArtifactMap = buildReviewArtifactMap(artifacts);

  let planGroupRendered = false;
  let reviewGroupRendered = false;

  return (
    <div className="flex flex-col gap-2">
      {stageResults.map((stageResult, idx) => {
        if (isPlanStage(stageResult.stage)) {
          if (planGroupRendered) return null;
          planGroupRendered = true;
          return (
            <TabbedPlanCard
              key="plan-group"
              planStages={planGroupStages}
              planArtifacts={planArtifactMap}
              runPrompt={run.prompt}
              enhancedPromptInput={artifacts["enhanced_prompt"] ?? run.prompt}
              settings={settings}
              runStatus={run.status}
            />
          );
        }

        if (isReviewStage(stageResult.stage)) {
          if (reviewGroupRendered) return null;
          reviewGroupRendered = true;
          return (
            <TabbedReviewCard
              key="review-group"
              reviewStages={reviewGroupStages}
              reviewArtifacts={reviewArtifactMap}
              runPrompt={run.prompt}
              enhancedPromptInput={artifacts["enhanced_prompt"] ?? run.prompt}
              settings={settings}
              runStatus={run.status}
              startedAt={
                isActiveStatus && run.currentStage && isReviewStage(run.currentStage as PipelineStage)
                  ? run.startedAt
                    ? parseUtcTimestamp(run.startedAt).getTime()
                    : undefined
                  : undefined
              }
            />
          );
        }

        return (
          <RichStageCard
            key={`${stageResult.stage}-${idx}`}
            stage={stageResult}
            runPrompt={run.prompt}
            enhancedPromptInput={artifacts["enhanced_prompt"] ?? run.prompt}
            promptEnhanceOutput={artifacts["enhanced_prompt"] ?? ""}
            planOutput={artifacts["plan"] ?? ""}
            planInputForAudit={artifacts["plan"] ?? ""}
            auditedPlanOutput={artifacts["plan_audit"] ?? ""}
            settings={settings}
            showPlanCard={false}
            startedAt={
              isActiveStatus && run.currentStage === stageResult.stage
                ? run.startedAt
                  ? parseUtcTimestamp(run.startedAt).getTime()
                  : undefined
                : undefined
            }
          />
        );
      })}
    </div>
  );
}
