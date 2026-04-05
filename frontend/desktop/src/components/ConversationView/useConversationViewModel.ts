import type { Dispatch, SetStateAction } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AgentSelection, ConversationDetail, WorkspaceInfo } from "../../types";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useFooterErrorHandler } from "../../hooks/useFooterErrorHandler";
import { usePipelineSession } from "../../hooks/usePipelineSession";
import { usePlanReview } from "../../hooks/usePlanReview";
import { useSettings } from "../../hooks/useSettings";
import {
  acceptPlan,
  getPipelineDebugLog,
  getPipelineState,
  redoReviewPipeline,
  resumePipeline,
  sendPlanEditFeedback,
  startPipeline,
  stopPipeline,
} from "../../lib/desktopApi";
import { parseAgentSelection } from "../../utils/agentSettings";
import { filterProvidersBySettings } from "../../utils/modelSettings";
import { sortProvidersByDisplayName } from "../shared/constants";
import type { PipelineMode } from "./ConversationComposer";

interface UseConversationViewModelParams {
  workspace: WorkspaceInfo;
  viewResetToken: number;
  activeConversation: ConversationDetail | null;
  onSetActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  pipelineMode: PipelineMode;
  onPipelineModeChange: (mode: PipelineMode) => void;
  onSendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  onStopConversation: () => Promise<void>;
}

export function useConversationViewModel({
  workspace,
  viewResetToken,
  activeConversation,
  onSetActiveConversation,
  pipelineMode,
  onPipelineModeChange,
  onSendPrompt,
  onStopConversation,
}: UseConversationViewModelParams) {
  const { providers, checkHealth } = useApiHealth();
  const { settings } = useSettings();
  const handleFooterError = useFooterErrorHandler();
  const [selectedAgent, setSelectedAgent] = useState<AgentSelection | null>(null);
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
      // Note: pipelineMode is now managed by the store, not reset here.
    }
  }, [activeConversation, pipeline, planReview, viewResetToken]);

  useEffect(() => {
    if (!activeConversation) {
      return;
    }

    let cancelled = false;
    void getPipelineState(workspace.path, activeConversation.summary.id).then((state) => {
      if (cancelled || !state) {
        return;
      }

      const isStillRunning = activeConversation.summary.status === "running";
      setPipelineConversationId(activeConversation.summary.id);
      pipeline.loadFromSaved(state, isStillRunning, activeConversation.summary.status);
      setPipelinePrompt(state.userPrompt);
      // Only default to "code" when loading a pipeline conversation if the
      // user has not already selected a mode for this conversation.  This
      // preserves the mode across view navigations (e.g. settings → home).
      if (pipelineMode === "auto") {
        onPipelineModeChange("code");
      }
    });

    void getPipelineDebugLog(workspace.path, activeConversation.summary.id).then((log) => {
      if (cancelled) {
        return;
      }
      pipeline.loadDebugLog(log);
    });

    return () => {
      cancelled = true;
    };
  }, [activeConversation, pipeline, workspace.path, onPipelineModeChange]);

  useEffect(() => {
    if (activeConversation) {
      setSelectedAgent(activeConversation.summary.agent);
      return;
    }

    if (!settings) {
      return;
    }

    const selectionIsValid = selectedAgent
      ? availableProviders.some((provider) => (
        provider.name === selectedAgent.provider
        && provider.models.includes(selectedAgent.model)
      ))
      : false;
    if (selectionIsValid) {
      return;
    }

    const defaultAgent = parseAgentSelection(settings.defaultAgent);
    if (defaultAgent) {
      const defaultProvider = availableProviders.find((provider) => provider.name === defaultAgent.provider);
      if (defaultProvider?.models.includes(defaultAgent.model)) {
        setSelectedAgent(defaultAgent);
        return;
      }
    }

    if (availableProviders[0]) {
      setSelectedAgent({
        provider: availableProviders[0].name,
        model: availableProviders[0].models[0] ?? "",
      });
    }
  }, [activeConversation, availableProviders, selectedAgent, settings]);

  const currentAgent = activeConversation?.summary.agent ?? selectedAgent;
  const activeRunning = activeConversation?.summary.status === "running";
  const pipelineDone = pipeline.stages.length > 0
    && !pipeline.running
    && !pipeline.awaitingReview
    && pipeline.stages.every((stage) => (
      stage.status === "completed" || stage.status === "failed" || stage.status === "stopped"
    ));
  const promptHistory = useMemo(
    () => activeConversation?.messages
      .filter((message) => message.role === "user")
      .map((message) => message.content) ?? [],
    [activeConversation],
  );

  const handleSend = useCallback(async (prompt: string) => {
    if (pipelineMode === "code") {
      pipeline.reset();
      planReview.reset();
      setPipelinePrompt(prompt);
      const detail = await startPipeline(workspace.path, prompt);
      setPipelineConversationId(detail.summary.id);
      // Set the active conversation so the header shows the title and
      // subsequent status events (e.g. orchestrator rename) can update it.
      onSetActiveConversation(detail);
      return;
    }

    const agent = activeConversation?.summary.agent ?? selectedAgent;
    if (!agent) {
      return;
    }

    await onSendPrompt(prompt, agent);
  }, [activeConversation, onSendPrompt, onSetActiveConversation, pipeline, pipelineMode, planReview, selectedAgent, workspace.path]);

  const handleStop = useCallback(async () => {
    if (pipelineConversationId) {
      await stopPipeline(workspace.path, pipelineConversationId);
      return;
    }

    await onStopConversation();
  }, [onStopConversation, pipelineConversationId, workspace.path]);

  const handleResume = useCallback(async () => {
    if (!pipelineConversationId) {
      return;
    }

    pipeline.softReset();
    planReview.reset();
    await resumePipeline(workspace.path, pipelineConversationId);
  }, [pipeline, pipelineConversationId, planReview, workspace.path]);

  const handleRedoReview = useCallback(async () => {
    if (!pipelineConversationId) {
      return;
    }

    pipeline.softReset();
    try {
      await redoReviewPipeline(workspace.path, pipelineConversationId);
    } catch (error) {
      console.error("[redo-review] Failed to start redo review:", error);
    }
  }, [pipeline, pipelineConversationId, workspace.path]);

  const handleNewPipeline = useCallback(() => {
    pipeline.reset();
    planReview.reset();
    setPipelineConversationId(null);
    setPipelinePrompt("");
    onPipelineModeChange("code");
  }, [pipeline, planReview, onPipelineModeChange]);

  return {
    availableProviders,
    currentAgent,
    setSelectedAgent,
    pipelinePrompt,
    pipeline,
    planReview,
    activeRunning,
    pipelineDone,
    promptHistory,
    handleSend,
    handleStop,
    handleResume,
    handleRedoReview,
    handleNewPipeline,
    handleFooterError,
  };
}
