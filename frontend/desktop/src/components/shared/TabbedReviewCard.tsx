import type { ReactNode } from "react";
import type { AppSettings, PipelineStage, StageResult } from "../../types";
import { TabbedParallelStageCard, type ParallelStageCardConfig } from "./TabbedParallelStageCard";

/** Returns true if a stage identifier belongs to the parallel review group. */
export function isReviewStage(stage: PipelineStage): boolean {
  return stage === "code_reviewer" || /^code_reviewer\d+$/.test(stage);
}

/** Derives a human-readable label from a review stage identifier. */
function reviewStageLabel(stage: string): string {
  if (stage === "code_reviewer") return "R1";
  const m = /^code_reviewer(\d+)$/.exec(stage);
  return m ? `R${m[1]}` : stage;
}

const REVIEW_CARD_CONFIG: ParallelStageCardConfig = {
  heading: "Code Review",
  headingBadgeClassName: "bg-[#ffb432]/25",
  countNoun: "reviewers",
  outputLabel: "Review",
  outputEmptyText: "No review output generated.",
  stageLabelFn: reviewStageLabel,
  singleArtifactKey: "review",
  artifactKeyPrefix: "review",
  activeSubTabBackgroundClassName: "bg-[#ffb432]/15",
  activeSubTabTextClassName: "text-[#ffb432]",
  activeSubTabDotClassName: "bg-[#ffb432]",
  outputBorderClassName: "border-orange-400/20",
  outputBackgroundClassName: "bg-orange-400/5",
};

interface TabbedReviewCardProps {
  /** All review stage results for this iteration. */
  reviewStages: StageResult[];
  /** Resolved review artifacts keyed by review_1, review_2, etc. or review. */
  reviewArtifacts: Record<string, string>;
  /** Original user prompt. */
  runPrompt: string;
  /** Enhanced prompt (or original if none). */
  enhancedPromptInput: string;
  settings: AppSettings | null;
  /** Absolute timestamp when the currently running stage started. */
  startedAt?: number;
  runStatus?: string;
}

export function TabbedReviewCard({
  reviewStages,
  reviewArtifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
  runStatus,
}: TabbedReviewCardProps): ReactNode {
  return (
    <TabbedParallelStageCard
      stages={reviewStages}
      artifacts={reviewArtifacts}
      runPrompt={runPrompt}
      enhancedPromptInput={enhancedPromptInput}
      settings={settings}
      startedAt={startedAt}
      runStatus={runStatus}
      config={REVIEW_CARD_CONFIG}
    />
  );
}
