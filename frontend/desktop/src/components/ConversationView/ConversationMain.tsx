import type { ReactNode } from "react";
import type { ConversationDetail } from "../../types";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";
import type { UsePipelineSessionReturn } from "../../hooks/usePipelineSession";
import { PipelineConversationView } from "./PipelineConversationView";
import { ConversationEmptyState } from "./ConversationEmptyState";
import { ConversationTranscript } from "./ConversationTranscript";

interface ConversationMainProps {
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  pipeline: UsePipelineSessionReturn;
  pipelinePrompt: string;
  planReviewPhase: PlanReviewPhase;
  onResume: () => Promise<void>;
  onStop: () => Promise<void>;
}

export function ConversationMain({
  activeConversation,
  activeDraft,
  pipeline,
  pipelinePrompt,
  planReviewPhase,
  onResume,
  onStop,
}: ConversationMainProps): ReactNode {
  if (pipeline.stages.length > 0 || pipeline.running || pipeline.userPrompt) {
    return (
      <PipelineConversationView
        userPrompt={pipelinePrompt || pipeline.userPrompt}
        stages={pipeline.stages}
        running={pipeline.running}
        currentStageName={pipeline.currentStageName}
        pipelineStartedAt={pipeline.pipelineStartedAt}
        onResume={onResume}
        onStop={onStop}
        planReviewPhase={planReviewPhase}
      />
    );
  }

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5">
      {activeConversation ? (
        <ConversationTranscript activeConversation={activeConversation} activeDraft={activeDraft} />
      ) : (
        <ConversationEmptyState />
      )}
    </div>
  );
}
