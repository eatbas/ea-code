import type { Dispatch, SetStateAction } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { AgentSelection, ConversationDetail, WorkspaceInfo } from "../../types";
import type { PendingImage } from "../../hooks/useImageAttachments";
import { useApiHealth } from "../../hooks/useApiHealth";
import { useFooterErrorHandler } from "../../hooks/useFooterErrorHandler";
import { usePipelineSession } from "../../hooks/usePipelineSession";
import { usePlanReview } from "../../hooks/usePlanReview";
import { useSettings } from "../../hooks/useSettings";
import { getThinkingOptions, KIMI_SWARM_PROMPT_PREFIX } from "../shared/constants";
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
  onSendPrompt: (prompt: string, agent: AgentSelection, pendingImages?: PendingImage[]) => Promise<void>;
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
  const { settings, saveSettings } = useSettings();
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
      const detail = await acceptPlan(workspace.path, pipelineConversationId);
      onSetActiveConversation((previous) => (
        previous?.summary.id === detail.summary.id ? detail : previous
      ));
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
      if (cancelled) {
        return;
      }

      if (!state) {
        // No pipeline state — this is a simple task conversation.
        // Always set the mode so the agent selector stays visible.
        onPipelineModeChange("simple");
        return;
      }

      const isStillRunning = activeConversation.summary.status === "running";
      setPipelineConversationId(activeConversation.summary.id);
      pipeline.loadFromSaved(state, isStillRunning, activeConversation.summary.status);
      setPipelinePrompt(state.userPrompt);
      // Always set "code" when loading a pipeline conversation.  The mode
      // is determined by backend state (the source of truth), not by
      // session-local storage, to avoid stale values from prior
      // auto-detection or effect re-runs.
      onPipelineModeChange("code");
    }).catch((error) => {
      console.warn("[pipeline] Failed to load pipeline state:", error);
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

  const kimiSwarmEnabled = settings?.kimiSwarmEnabled ?? false;
  const kimiRalphIterations = settings?.kimiMaxRalphIterations ?? 1;
  const isKimi = currentAgent?.provider === "kimi";
  const isResume = Boolean(activeConversation?.summary.lastProviderSessionRef);
  const [redoSwarm, setRedoSwarm] = useState(false);

  const handleSend = useCallback(async (prompt: string, pendingImages?: PendingImage[]) => {
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

    const effectivePrompt = (redoSwarm && isKimi && kimiSwarmEnabled && isResume)
      ? KIMI_SWARM_PROMPT_PREFIX + prompt
      : prompt;
    if (redoSwarm) setRedoSwarm(false);
    await onSendPrompt(effectivePrompt, agent, pendingImages);
  }, [activeConversation, isKimi, isResume, kimiSwarmEnabled, onSendPrompt, onSetActiveConversation, pipeline, pipelineMode, planReview, redoSwarm, selectedAgent, workspace.path]);

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

  const thinkingLevel = useMemo(() => {
    if (!settings || !currentAgent) return "";
    const key = `${currentAgent.provider}:${currentAgent.model}`;
    return settings.providerThinking[key] ?? "";
  }, [settings, currentAgent]);

  const thinkingOptions = useMemo(() => {
    if (!currentAgent) return undefined;
    return getThinkingOptions(currentAgent.provider, currentAgent.model);
  }, [currentAgent]);

  const handleThinkingChange = useCallback((value: string) => {
    if (!settings || !currentAgent) return;
    const key = `${currentAgent.provider}:${currentAgent.model}`;
    const updated = { ...settings.providerThinking };
    if (value) {
      updated[key] = value;
    } else {
      delete updated[key];
    }
    void saveSettings({ ...settings, providerThinking: updated });
  }, [settings, currentAgent, saveSettings]);

  const handleSwarmChange = useCallback((value: string) => {
    if (!settings) return;
    void saveSettings({ ...settings, kimiSwarmEnabled: value === "enabled" });
  }, [settings, saveSettings]);

  const handleRalphIterationsChange = useCallback((value: string) => {
    if (!settings) return;
    const parsed = value ? parseInt(value, 10) : 1;
    void saveSettings({ ...settings, kimiMaxRalphIterations: parsed });
  }, [settings, saveSettings]);

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
    thinkingLevel,
    thinkingOptions,
    handleSend,
    handleStop,
    handleResume,
    handleRedoReview,
    handleNewPipeline,
    handleThinkingChange,
    isKimi,
    isResume,
    kimiSwarmEnabled,
    kimiRalphIterations,
    redoSwarm,
    setRedoSwarm,
    handleSwarmChange,
    handleRalphIterationsChange,
    handleFooterError,
  };
}
