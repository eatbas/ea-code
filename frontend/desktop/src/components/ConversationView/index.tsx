import type { Dispatch, ReactNode, SetStateAction } from "react";
import type { AgentSelection, ConversationDetail, WorkspaceInfo } from "../../types";
import type { PendingImage } from "../../hooks/useImageAttachments";
import type { PipelineMode } from "./ConversationComposer";
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
  onSetActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  activeDraft: string;
  activePromptDraft: string;
  pipelineMode: PipelineMode;
  onPipelineModeChange: (mode: PipelineMode) => void;
  sending: boolean;
  stopping: boolean;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onPromptDraftChange: (prompt: string) => void;
  onSendPrompt: (prompt: string, agent: AgentSelection, pendingImages?: PendingImage[]) => Promise<void>;
  onStopConversation: () => Promise<void>;
}

export function ConversationView({
  workspace,
  sidecarReady,
  viewResetToken,
  activeConversation,
  onSetActiveConversation,
  activeDraft,
  activePromptDraft,
  pipelineMode,
  onPipelineModeChange,
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
    onSetActiveConversation,
    pipelineMode,
    onPipelineModeChange,
    onSendPrompt,
    onStopConversation,
  });

  const reviewingPlan = !viewModel.pipeline.running
    && (viewModel.planReview.phase === "reviewing"
      || viewModel.planReview.phase === "editing");

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
            pipelineMode={pipelineMode}
            pipelineDone={viewModel.pipelineDone}
            sidecarReady={sidecarReady}
            thinkingLevel={viewModel.thinkingLevel}
            thinkingOptions={viewModel.thinkingOptions}
            workspacePath={workspace.path}
            conversationId={activeConversation?.summary.id ?? null}
            onPipelineModeChange={onPipelineModeChange}
            onAgentChange={viewModel.setSelectedAgent}
            onThinkingChange={viewModel.handleThinkingChange}
            isKimi={viewModel.isKimi}
            kimiSwarmEnabled={viewModel.kimiSwarmEnabled}
            kimiRalphIterations={viewModel.kimiRalphIterations}
            isResume={viewModel.isResume}
            redoSwarm={viewModel.redoSwarm}
            onRedoSwarmChange={viewModel.setRedoSwarm}
            onSwarmChange={viewModel.handleSwarmChange}
            onRalphIterationsChange={viewModel.handleRalphIterationsChange}
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
