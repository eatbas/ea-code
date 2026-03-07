import type { ReactNode } from "react";
import type { StageResult, PipelineStage, StageStatus } from "../types";

interface RunTimelineProps {
  stages: StageResult[];
  currentStage?: PipelineStage;
  iteration: number;
}

/** Display labels for each pipeline stage. */
const STAGE_LABELS: Record<PipelineStage, string> = {
  generate: "Generate",
  diff_after_generate: "Diff",
  review: "Review",
  fix: "Fix",
  diff_after_fix: "Diff",
  validate: "Validate",
  judge: "Judge",
};

/** All stages in pipeline order. */
const STAGE_ORDER: PipelineStage[] = [
  "generate",
  "diff_after_generate",
  "review",
  "fix",
  "diff_after_fix",
  "validate",
  "judge",
];

/** Status icon component for a pipeline stage. */
function StageIcon({ status }: { status: StageStatus }): ReactNode {
  switch (status) {
    case "running":
      return (
        <svg
          className="animate-spin h-4 w-4 text-[#6366f1]"
          xmlns="http://www.w3.org/2000/svg"
          fill="none"
          viewBox="0 0 24 24"
        >
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
          />
        </svg>
      );
    case "completed":
      return (
        <svg
          className="h-4 w-4 text-[#22c55e]"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <polyline points="20 6 9 17 4 12" />
        </svg>
      );
    case "failed":
      return (
        <svg
          className="h-4 w-4 text-[#ef4444]"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      );
    case "skipped":
      return (
        <svg
          className="h-4 w-4 text-[#9898b0]"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <line x1="5" y1="12" x2="19" y2="12" />
        </svg>
      );
    case "waiting_for_input":
      return (
        <svg
          className="h-4 w-4 text-[#f59e0b] animate-pulse"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
      );
    default:
      // pending
      return (
        <div className="h-4 w-4 rounded-full border-2 border-[#9898b0]" />
      );
  }
}

/** Colour for the left border accent based on stage status. */
function borderColour(status: StageStatus): string {
  switch (status) {
    case "running":
      return "border-l-[#6366f1]";
    case "completed":
      return "border-l-[#22c55e]";
    case "failed":
      return "border-l-[#ef4444]";
    case "waiting_for_input":
      return "border-l-[#f59e0b]";
    default:
      return "border-l-[#2e2e48]";
  }
}

/** Horizontal timeline showing the progress through pipeline stages. */
export function RunTimeline({ stages, currentStage, iteration }: RunTimelineProps): ReactNode {
  function getStageStatus(stage: PipelineStage): StageStatus {
    const result = stages.find((s) => s.stage === stage);
    if (result?.status === "waiting_for_input") return "waiting_for_input";
    if (currentStage === stage && result?.status !== "completed") return "running";
    return result?.status ?? "pending";
  }

  function getDuration(stage: PipelineStage): number | undefined {
    const result = stages.find((s) => s.stage === stage);
    return result?.status === "completed" ? result.durationMs : undefined;
  }

  return (
    <div className="bg-[#1a1a24] border-b border-[#2e2e48] px-4 py-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs font-medium text-[#9898b0]">Pipeline Stages</span>
        <span className="text-xs text-[#9898b0]">Iteration {iteration}</span>
      </div>
      <div className="flex gap-2">
        {STAGE_ORDER.map((stage) => {
          const status = getStageStatus(stage);
          const duration = getDuration(stage);

          return (
            <div
              key={stage}
              className={`flex-1 rounded border border-[#2e2e48] border-l-2 ${borderColour(status)} bg-[#0f0f14] px-2 py-2`}
            >
              <div className="flex items-center gap-1.5 mb-1">
                <StageIcon status={status} />
                <span className="text-xs font-medium text-[#e4e4ed] truncate">
                  {STAGE_LABELS[stage]}
                </span>
              </div>
              {duration !== undefined && (
                <span className="text-[10px] text-[#9898b0]">
                  {(duration / 1000).toFixed(1)}s
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
