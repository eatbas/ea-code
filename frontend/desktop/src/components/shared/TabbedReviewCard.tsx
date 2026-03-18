import type { ReactNode } from "react";
import type { AppSettings, PipelineStage, StageResult } from "../../types";
import { TabbedParallelStageCard, type ParallelStageCardConfig } from "./TabbedParallelStageCard";

/** Stages considered part of the parallel review group. */
const REVIEW_STAGES = new Set<PipelineStage>(["code_reviewer", "code_reviewer2", "code_reviewer3"]);

const REVIEW_LABEL_MAP: Record<string, string> = {
  code_reviewer: "Review 1",
  code_reviewer2: "Review 2",
  code_reviewer3: "Review 3",
};

const REVIEW_CARD_CONFIG: ParallelStageCardConfig = {
  heading: "Code Review",
  headingBadgeClassName: "bg-[#ffb432]/25",
  countNoun: "reviewers",
  outputLabel: "Review",
  outputEmptyText: "No review output generated.",
  stageLabelMap: REVIEW_LABEL_MAP,
  singleArtifactKey: "review",
  artifactKeyPrefix: "review",
  activeSubTabBackgroundClassName: "bg-[#ffb432]/15",
  activeSubTabTextClassName: "text-[#ffb432]",
  activeSubTabDotClassName: "bg-[#ffb432]",
  outputBorderClassName: "border-orange-400/20",
  outputBackgroundClassName: "bg-orange-400/5",
};

export function isReviewStage(stage: PipelineStage): boolean {
  return REVIEW_STAGES.has(stage);
}

interface TabbedReviewCardProps {
  /** All review stage results (code_reviewer, code_reviewer2, code_reviewer3) for this iteration. */
  reviewStages: StageResult[];
  /** Resolved review artifacts keyed by review_1, review_2, review_3 or review. */
  reviewArtifacts: Record<string, string>;
  /** Original user prompt. */
  runPrompt: string;
  /** Enhanced prompt (or original if none). */
  enhancedPromptInput: string;
  settings: AppSettings | null;
  /** Absolute timestamp when the currently running stage started. */
  startedAt?: number;
}

export function TabbedReviewCard({
  reviewStages,
  reviewArtifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
}: TabbedReviewCardProps): ReactNode {
  return (
    <TabbedParallelStageCard
      stages={reviewStages}
      artifacts={reviewArtifacts}
      runPrompt={runPrompt}
      enhancedPromptInput={enhancedPromptInput}
      settings={settings}
      startedAt={startedAt}
      config={REVIEW_CARD_CONFIG}
    />
  );
}
