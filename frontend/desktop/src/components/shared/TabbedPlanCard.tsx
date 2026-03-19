import type { ReactNode } from "react";
import type { AppSettings, PipelineStage, StageResult } from "../../types";
import { TabbedParallelStageCard, type ParallelStageCardConfig } from "./TabbedParallelStageCard";

/** Returns true if a stage identifier belongs to the parallel planning group. */
export function isPlanStage(stage: PipelineStage): boolean {
  return stage === "plan" || /^plan\d+$/.test(stage);
}

/** Derives a human-readable label from a plan stage identifier. */
function planStageLabel(stage: string): string {
  if (stage === "plan") return "P1";
  const m = /^plan(\d+)$/.exec(stage);
  return m ? `P${m[1]}` : stage;
}

const PLAN_CARD_CONFIG: ParallelStageCardConfig = {
  heading: "Planning",
  headingBadgeClassName: "bg-[#40c4ff]/25",
  countNoun: "planners",
  outputLabel: "Plan",
  outputEmptyText: "No valid plan output generated.",
  stageLabelFn: planStageLabel,
  singleArtifactKey: "plan",
  artifactKeyPrefix: "plan",
  activeSubTabBackgroundClassName: "bg-[#40c4ff]/15",
  activeSubTabTextClassName: "text-[#40c4ff]",
  activeSubTabDotClassName: "bg-[#40c4ff]",
  outputBorderClassName: "border-sky-400/20",
  outputBackgroundClassName: "bg-sky-400/5",
};

interface TabbedPlanCardProps {
  /** All plan stage results for this iteration. */
  planStages: StageResult[];
  /** Resolved plan artifacts keyed by plan_1, plan_2, etc. or plan. */
  planArtifacts: Record<string, string>;
  /** Original user prompt. */
  runPrompt: string;
  /** Enhanced prompt (or original if none). */
  enhancedPromptInput: string;
  settings: AppSettings | null;
  /** Absolute timestamp when the currently running stage started. */
  startedAt?: number;
  runStatus?: string;
}

export function TabbedPlanCard({
  planStages,
  planArtifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
  runStatus,
}: TabbedPlanCardProps): ReactNode {
  return (
    <TabbedParallelStageCard
      stages={planStages}
      artifacts={planArtifacts}
      runPrompt={runPrompt}
      enhancedPromptInput={enhancedPromptInput}
      settings={settings}
      startedAt={startedAt}
      runStatus={runStatus}
      config={PLAN_CARD_CONFIG}
    />
  );
}
