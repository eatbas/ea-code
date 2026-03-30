import type { ReactNode } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  AgentSelection,
  ConversationDetail,
  WorkspaceInfo,
} from "../../types";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useSettings } from "../../hooks/useSettings";
import { ConversationComposer, type PipelineMode } from "./ConversationComposer";
import { formatAssistantLabel, sortProvidersByDisplayName } from "../shared/constants";
import { WorkspaceFooter } from "../shared/WorkspaceFooter";
import { useFooterErrorHandler } from "../../hooks/useFooterErrorHandler";
import { filterProvidersBySettings } from "../../utils/modelSettings";
import { parseAgentSelection } from "../../utils/agentSettings";
import { usePipelineSession } from "../../hooks/usePipelineSession";
import { usePlanReview } from "../../hooks/usePlanReview";
import {
  acceptPlan,
  getPipelineState,
  resumePipeline,
  sendPlanEditFeedback,
  startPipeline,
  stopPipeline,
} from "../../lib/desktopApi";
import { PipelineConversationView } from "./PipelineConversationView";
import { PlanReviewCard } from "./PlanReviewCard";

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
  const { providers, checkHealth } = useApiHealth();
  const { settings } = useSettings();
  const [selectedAgent, setSelectedAgent] = useState<AgentSelection | null>(null);
  const [pipelineMode, setPipelineMode] = useState<PipelineMode>("auto");
  const [pipelinePrompt, setPipelinePrompt] = useState<string>("");
  const [pipelineConversationId, setPipelineConversationId] = useState<string | null>(null);
  const prevConversationIdRef = useRef<string | null>(null);
  const prevResetTokenRef = useRef(viewResetToken);
  const pipeline = usePipelineSession(pipelineConversationId);

  const planReview = usePlanReview({
    awaitingReview: pipeline.awaitingReview,
    running: pipeline.running,
    onAccept: async () => {
      if (!pipelineConversationId) return;
      await acceptPlan(workspace.path, pipelineConversationId);
    },
    onSubmitFeedback: async (feedback: string) => {
      if (!pipelineConversationId) return;
      await sendPlanEditFeedback(workspace.path, pipelineConversationId, feedback);
    },
  });

  const availableProviders = useMemo(
    () => sortProvidersByDisplayName(filterProvidersBySettings(providers, settings)),
    [providers, settings],
  );

  useEffect(() => {
    checkHealth();
  }, [checkHealth, workspace.path]);

  // Reset selection and pipeline state when leaving an existing conversation
  // so the default agent is re-applied for the next new conversation.
  // Also reset when viewResetToken changes (new-conversation or delete pressed
  // while activeConversation is already null — React can't detect null→null).
  useEffect(() => {
    const currentId = activeConversation?.summary.id ?? null;
    const prevId = prevConversationIdRef.current;
    const tokenChanged = viewResetToken !== prevResetTokenRef.current;
    prevConversationIdRef.current = currentId;
    prevResetTokenRef.current = viewResetToken;

    if ((prevId !== null && currentId !== prevId) || tokenChanged) {
      setSelectedAgent(null);
      pipeline.reset();
      planReview.reset();
      setPipelinePrompt("");
      setPipelineConversationId(null);
      setPipelineMode("auto");
    }
  }, [activeConversation, viewResetToken]);

  // Load saved pipeline state when opening a conversation.
  useEffect(() => {
    if (!activeConversation) return;
    let cancelled = false;
    void getPipelineState(workspace.path, activeConversation.summary.id).then((state) => {
      if (cancelled || !state) return;
      const isStillRunning = activeConversation.summary.status === "running";
      // Set the conversation ID first so the event filter ref is populated
      // before loadFromSaved potentially marks the pipeline as running.
      setPipelineConversationId(activeConversation.summary.id);
      pipeline.loadFromSaved(state, isStillRunning, activeConversation.summary.status);
      setPipelinePrompt(state.userPrompt);
      setPipelineMode("code");
    });
    return () => { cancelled = true; };
  }, [activeConversation?.summary.id, workspace.path]);

  useEffect(() => {
    if (activeConversation) {
      setSelectedAgent(activeConversation.summary.agent);
      return;
    }

    // Wait for settings to load before making any selection so we
    // don't fall back to the first provider before the default is known.
    if (!settings) return;

    const selectionIsValid = selectedAgent
      ? availableProviders.some((provider) => (
        provider.name === selectedAgent.provider
        && provider.models.includes(selectedAgent.model)
      ))
      : false;

    if (selectionIsValid) return;

    // Try the configured default agent from settings.
    const defaultAgent = parseAgentSelection(settings.defaultAgent);
    if (defaultAgent) {
      const defaultProvider = availableProviders.find(
        (p) => p.name === defaultAgent.provider,
      );
      if (defaultProvider?.models.includes(defaultAgent.model)) {
        setSelectedAgent(defaultAgent);
        return;
      }
    }

    // Fall back to first available provider/model.
    if (availableProviders[0]) {
      setSelectedAgent({
        provider: availableProviders[0].name,
        model: availableProviders[0].models[0] ?? "",
      });
    }
  }, [activeConversation, availableProviders, selectedAgent, settings]);

  const activeRunning = activeConversation?.summary.status === "running";

  const handleSend = useCallback(async (prompt: string) => {
    if (pipelineMode === "code") {
      pipeline.reset();
      planReview.reset();
      setPipelinePrompt(prompt);
      const detail = await startPipeline(workspace.path, prompt);
      setPipelineConversationId(detail.summary.id);
      return;
    }
    const agent = activeConversation?.summary.agent ?? selectedAgent;
    if (!agent) {
      return;
    }
    await onSendPrompt(prompt, agent);
  }, [pipelineMode, activeConversation, selectedAgent, onSendPrompt, workspace.path, pipeline, planReview]);

  const handleStop = useCallback(async () => {
    if (pipelineConversationId) {
      await stopPipeline(workspace.path, pipelineConversationId);
      return;
    }
    await onStopConversation();
  }, [pipeline, pipelineConversationId, workspace.path, onStopConversation]);

  const handleResume = useCallback(async () => {
    if (!pipelineConversationId) return;
    pipeline.softReset();
    planReview.reset();
    await resumePipeline(workspace.path, pipelineConversationId);
  }, [pipelineConversationId, workspace.path, pipeline, planReview]);

  const handleNewPipeline = useCallback(() => {
    pipeline.reset();
    planReview.reset();
    setPipelineConversationId(null);
    setPipelinePrompt("");
    setPipelineMode("code");
  }, [pipeline, planReview]);

  // awaiting_review is NOT "done" — it's a special intermediate state.
  const pipelineDone = pipeline.stages.length > 0 && !pipeline.running
    && !pipeline.awaitingReview
    && pipeline.stages.every((s) => (
      s.status === "completed" || s.status === "failed" || s.status === "stopped"
    ));

  const handleFooterError = useFooterErrorHandler();

  const promptHistory = useMemo(
    () => activeConversation?.messages
      .filter((message) => message.role === "user")
      .map((message) => message.content) ?? [],
    [activeConversation],
  );

  return (
    <div className="flex h-full min-h-0 bg-surface">
      <div className="flex min-h-0 flex-1 flex-col">
        <div className="border-b border-edge bg-[linear-gradient(180deg,var(--color-input-bg)_0%,var(--color-surface)_100%)] px-5 py-4">
          <p className="text-lg font-semibold text-fg">
            {activeConversation?.summary.title ?? "New conversation"}
          </p>
          {!activeConversation && (
            <p className="mt-1 text-sm text-fg-muted">{workspace.path}</p>
          )}
        </div>

        {workspace.isGitRepo && workspace.maestroIgnored === false && (
          <div className="border-b border-warning-border bg-new-btn-bg-hover px-5 py-3 text-sm text-warning-text">
            `.maestro/` is not currently ignored in this repository. Maestro will attempt to add it to `.gitignore`.
          </div>
        )}

        {pipeline.stages.length > 0 || pipeline.running || pipeline.userPrompt ? (
          <PipelineConversationView
            userPrompt={pipelinePrompt || pipeline.userPrompt}
            stages={pipeline.stages}
            running={pipeline.running}
            currentStageName={pipeline.currentStageName}
            pipelineStartedAt={pipeline.pipelineStartedAt}
            onResume={handleResume}
            onStop={handleStop}
            planReviewPhase={planReview.phase}
          />
        ) : (
        <div className="min-h-0 flex-1 overflow-y-auto px-5 py-5">
          {activeConversation ? (
            <div className="mx-auto flex w-full max-w-4xl flex-col gap-4">
              {activeConversation.messages.map((message) => (
                <div
                  key={message.id}
                  className={`flex max-w-3xl flex-col ${message.role === "user" ? "ml-auto items-end" : "mr-auto items-end"}`}
                >
                  <div
                    className={`rounded-2xl px-4 py-3 text-sm leading-6 ${
                      message.role === "user"
                        ? "border border-edge-strong bg-elevated text-fg"
                        : "border border-edge bg-panel text-fg"
                    }`}
                  >
                    <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                      {message.role === "assistant" && activeConversation
                        ? `Assistant - ${formatAssistantLabel(
                          activeConversation.summary.agent.provider,
                          activeConversation.summary.agent.model,
                        )}`
                        : message.role}
                    </p>
                    <p className="whitespace-pre-wrap">{message.content}</p>
                  </div>
                  <p className="mt-1 px-1 text-[10px] text-fg-muted">
                    {new Date(message.createdAt).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
                  </p>
                </div>
              ))}
              {activeDraft && (
                <div className="mr-auto max-w-3xl rounded-2xl border border-dashed border-edge-strong bg-input-bg px-4 py-3 text-sm leading-6 text-fg">
                  <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                    {activeConversation
                      ? `Assistant - ${formatAssistantLabel(
                        activeConversation.summary.agent.provider,
                        activeConversation.summary.agent.model,
                      )}`
                      : "Assistant"}
                  </p>
                  <p className="whitespace-pre-wrap">{activeDraft}</p>
                </div>
              )}
              {activeConversation.summary.error && activeConversation.summary.status === "failed" && (
                <div className="mr-auto max-w-3xl rounded-2xl border border-error-border bg-error-bg px-4 py-3 text-sm text-error-text">
                  {activeConversation.summary.error}
                </div>
              )}
            </div>
          ) : (
            <div className="mx-auto flex h-full max-w-3xl items-center justify-center">
              <div className="rounded-3xl border border-edge bg-panel px-8 py-10 text-center shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
                <img src="/logo_2.png" alt="Maestro logo" className="mx-auto mb-4 h-64 w-64 object-contain" />
                <p className="text-xl font-semibold text-fg">Start a new conversation</p>
                <p className="mt-2 text-sm text-fg-muted">
                  Pick an agent in the composer and send the first prompt to start a conversation.
                </p>
              </div>
            </div>
          )}
        </div>
        )}

        {(planReview.phase === "reviewing" || planReview.phase === "editing")
        ? (
          <PlanReviewCard
            planText={pipeline.stages.find((s) => s.stageName === "Plan Merge")?.text ?? ""}
            phase={planReview.phase}
            countdown={planReview.countdown}
            onAccept={planReview.accept}
            onEdit={planReview.startEdit}
            onSubmitFeedback={planReview.submitFeedback}
          />
        ) : (
          <ConversationComposer
            providers={availableProviders}
            agent={activeConversation?.summary.agent ?? selectedAgent}
            prompt={activePromptDraft}
            promptHistory={promptHistory}
            locked={Boolean(activeConversation)}
            sending={sending}
            stopping={stopping}
            activeRunning={Boolean(activeRunning)}
            pipelineRunning={pipeline.running}
            pipelineMode={pipelineMode}
            pipelineDone={pipelineDone}
            sidecarReady={sidecarReady}
            onPipelineModeChange={setPipelineMode}
            onAgentChange={setSelectedAgent}
            onPromptChange={onPromptDraftChange}
            onSend={handleSend}
            onStop={handleStop}
            onResumePipeline={handleResume}
            onNewPipeline={handleNewPipeline}
            planReviewPhase={planReview.phase}
          />
        )}

        <div className="px-5 pb-3 pt-0">
          <div className="mx-auto flex w-full max-w-5xl">
            <WorkspaceFooter
              path={workspace.path}
              onOpenProjectFolder={onOpenProjectFolder}
              onOpenInVsCode={onOpenInVsCode}
              onError={handleFooterError}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
