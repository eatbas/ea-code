import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, RunSummary, RunEvent, PipelineStage, StageResult, StageStatus } from "../types";
import { parseUtcTimestamp } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { isActiveStatusValue, isTerminalStatusValue } from "../utils/statusHelpers";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { StageCard } from "./shared/StageCard";
import { RichStageCard } from "./shared/RichStageCard";
import { TabbedPlanCard, isPlanStage } from "./shared/TabbedPlanCard";
import { ResultCard, buildStageRowsFromEvents, computeDuration } from "./shared/ResultCard";

interface RunCardProps {
  run: RunSummary;
  settings: AppSettings | null;
  /** When true, hides the user prompt bubble (rendered separately in message-driven view). */
  hidePromptBubble?: boolean;
}

/** Converts RunEvent array to StageResult array for display purposes.
 *  Only includes stages that have completed (have a stage_end event).
 */
function eventsToStageResults(events: RunEvent[]): StageResult[] {
  const stageMap = new Map<string, { stage: string; status: StageStatus; output: string; durationMs: number }>();

  for (const event of events) {
    if (event.type === "stage_end" && event.stage) {
      const key = `${event.stage}-${event.iteration}`;
      const existing = stageMap.get(key);
      if (!existing) {
        stageMap.set(key, {
          stage: event.stage,
          status: event.status as StageStatus,
          output: "", // Output not stored in new system
          durationMs: event.durationMs ?? 0,
        });
      }
    }
  }

  return Array.from(stageMap.values()).map((s) => ({
    stage: s.stage as PipelineStage,
    status: s.status,
    output: s.output,
    durationMs: s.durationMs,
  }));
}

/** Displays a single historical run with full step-by-step timeline.
 *  Events are lazy-loaded when the card is expanded.
 */
export function RunCard({ run, settings, hidePromptBubble }: RunCardProps): ReactNode {
  const [events, setEvents] = useState<RunEvent[] | null>(null);
  const [artifacts, setArtifacts] = useState<Record<string, string>>({});
  const [loadingEvents, setLoadingEvents] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);

  const loadEvents = useCallback(async () => {
    if (events || loadingEvents) return; // Already loaded or loading
    setLoadingEvents(true);
    try {
      const [runEvents, runArtifacts] = await Promise.all([
        invoke<RunEvent[]>("get_run_events", { runId: run.id }),
        invoke<Record<string, string>>("get_run_artifacts", { runId: run.id }),
      ]);
      setEvents(runEvents);
      setArtifacts(runArtifacts);
    } catch (e) {
      console.error("Failed to load events:", e);
    } finally {
      setLoadingEvents(false);
    }
  }, [events, loadingEvents, run.id]);

  // Load events when expanded
  useEffect(() => {
    if (isExpanded) {
      loadEvents();
    }
  }, [isExpanded, loadEvents]);

  // For active runs, always show full details (events will be loaded)
  const isTerminalStatus = isTerminalStatusValue(run.status);
  const isActiveStatus = isActiveStatusValue(run.status);
  const activeStage = run.currentStage ?? (run.status === "running" ? "prompt_enhance" : undefined);

  // Auto-expand active runs
  useEffect(() => {
    if (isActiveStatus) {
      setIsExpanded(true);
    }
  }, [isActiveStatus]);

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
              {run.status === "running" && activeStage !== "plan_audit" && (
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
  const planArtifactMap: Record<string, string> = {};
  if (artifacts["plan"]) planArtifactMap["plan"] = artifacts["plan"];
  if (artifacts["plan_1"]) planArtifactMap["plan_1"] = artifacts["plan_1"];
  if (artifacts["plan_2"]) planArtifactMap["plan_2"] = artifacts["plan_2"];
  if (artifacts["plan_3"]) planArtifactMap["plan_3"] = artifacts["plan_3"];

  let planGroupRendered = false;

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
            reviewOutput={artifacts["review"] ?? ""}
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
