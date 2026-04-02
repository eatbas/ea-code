import type { ReactNode } from "react";
import type { AgentSelection, ConversationDetail, WorkspaceInfo } from "../../types";
import { PlanReviewCard } from "./PlanReviewCard";
import { ConversationComposer } from "./ConversationComposer";
import { ConversationFooter } from "./ConversationFooter";
import { ConversationHeader } from "./ConversationHeader";
import { ConversationMain } from "./ConversationMain";
import { useConversationViewModel } from "./useConversationViewModel";

interface ConversationViewProps {
  workspace: WorkspaceInfo;
  sidecarReady: boolean | null;
  viewResetToken: number;
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  activePromptDraft: string;
  sending: boolean;
  stopping: boolean;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onPromptDraftChange: (prompt: string) => void;
  onSendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  onStopConversation: () => Promise<void>;
}

export function ConversationView({
  workspace,
  sidecarReady,
  viewResetToken,
  activeConversation,
  activeDraft,
  activePromptDraft,
  sending,
  stopping,
  onOpenProjectFolder,
  onOpenInVsCode,
  onPromptDraftChange,
  onSendPrompt,
  onStopConversation,
}: ConversationViewProps): ReactNode {
  const viewModel = useConversationViewModel({
    workspace,
    viewResetToken,
    activeConversation,
    onSendPrompt,
    onStopConversation,
  });

  const reviewingPlan = viewModel.planReview.phase === "reviewing"
    || viewModel.planReview.phase === "editing";

  return (
    <div className="flex h-full min-h-0 bg-surface">
      <div className="flex min-h-0 flex-1 flex-col">
        <ConversationHeader workspace={workspace} activeConversation={activeConversation} />
        <ConversationMain
          activeConversation={activeConversation}
          activeDraft={activeDraft}
          pipeline={viewModel.pipeline}
          pipelinePrompt={viewModel.pipelinePrompt}
          planReviewPhase={viewModel.planReview.phase}
          onResume={viewModel.handleResume}
          onRedoReview={viewModel.handleRedoReview}
          onStop={viewModel.handleStop}
        />

        {reviewingPlan ? (
          <PlanReviewCard
            planText={viewModel.pipeline.stages.find((stage) => stage.stageName === "Plan Merge")?.text ?? ""}
            phase={viewModel.planReview.phase}
            countdown={viewModel.planReview.countdown}
            onAccept={viewModel.planReview.accept}
            onEdit={viewModel.planReview.startEdit}
            onSubmitFeedback={viewModel.planReview.submitFeedback}
          />
        ) : (
          <ConversationComposer
            providers={viewModel.availableProviders}
            agent={viewModel.currentAgent}
            prompt={activePromptDraft}
            promptHistory={viewModel.promptHistory}
            locked={Boolean(activeConversation)}
            sending={sending}
            stopping={stopping}
            activeRunning={Boolean(viewModel.activeRunning)}
            pipelineRunning={viewModel.pipeline.running}
            pipelineMode={viewModel.pipelineMode}
            pipelineDone={viewModel.pipelineDone}
            sidecarReady={sidecarReady}
            onPipelineModeChange={viewModel.setPipelineMode}
            onAgentChange={viewModel.setSelectedAgent}
            onPromptChange={onPromptDraftChange}
            onSend={viewModel.handleSend}
            onStop={viewModel.handleStop}
            onResumePipeline={viewModel.handleResume}
            onNewPipeline={viewModel.handleNewPipeline}
            planReviewPhase={viewModel.planReview.phase}
          />
        )}

        <ConversationFooter
          path={workspace.path}
          onOpenProjectFolder={onOpenProjectFolder}
          onOpenInVsCode={onOpenInVsCode}
          onError={viewModel.handleFooterError}
        />
      </div>
    </div>
  );
}
