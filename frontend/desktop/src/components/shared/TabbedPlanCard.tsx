import type { ReactNode } from "react";
import type { AppSettings, PipelineStage, StageResult } from "../../types";
import { TabbedParallelStageCard, type ParallelStageCardConfig } from "./TabbedParallelStageCard";

/** Stages considered part of the parallel planning group. */
const PLAN_STAGES = new Set<PipelineStage>(["plan", "plan2", "plan3"]);

const PLAN_LABEL_MAP: Record<string, string> = {
  plan: "Plan 1",
  plan2: "Plan 2",
  plan3: "Plan 3",
};

const PLAN_CARD_CONFIG: ParallelStageCardConfig = {
  heading: "Planning",
  headingBadgeClassName: "bg-[#40c4ff]/25",
  countNoun: "planners",
  outputLabel: "Plan",
  outputEmptyText: "No valid plan output generated.",
  stageLabelMap: PLAN_LABEL_MAP,
  singleArtifactKey: "plan",
  artifactKeyPrefix: "plan",
  activeSubTabBackgroundClassName: "bg-[#40c4ff]/15",
  activeSubTabTextClassName: "text-[#40c4ff]",
  activeSubTabDotClassName: "bg-[#40c4ff]",
  outputBorderClassName: "border-sky-400/20",
  outputBackgroundClassName: "bg-sky-400/5",
};

export function isPlanStage(stage: PipelineStage): boolean {
  return PLAN_STAGES.has(stage);
}

interface TabbedPlanCardProps {
  /** All plan stage results (plan, plan2, plan3) for this iteration. */
  planStages: StageResult[];
  /** Resolved plan artifacts keyed by plan_1, plan_2, plan_3 or plan. */
  planArtifacts: Record<string, string>;
  /** Original user prompt. */
  runPrompt: string;
  /** Enhanced prompt (or original if none). */
  enhancedPromptInput: string;
  settings: AppSettings | null;
  /** Absolute timestamp when the currently running stage started. */
  startedAt?: number;
}

export function TabbedPlanCard({
  planStages,
  planArtifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
}: TabbedPlanCardProps): ReactNode {
  return (
    <TabbedParallelStageCard
      stages={planStages}
      artifacts={planArtifacts}
      runPrompt={runPrompt}
      enhancedPromptInput={enhancedPromptInput}
      settings={settings}
      startedAt={startedAt}
      config={PLAN_CARD_CONFIG}
    />
  );
}
