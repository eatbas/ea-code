import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ConversationStatus,
  PipelineDebugLogEvent,
  ConversationStatusEvent,
  PipelineState,
  PipelineStageStatusEvent,
  PipelineStageOutputDelta,
} from "../types";
import { CONVERSATION_EVENTS, PIPELINE_EVENTS } from "../constants/events";
import { useTauriEventListeners } from "./useTauriEventListeners";
import type { StageStatus } from "../components/ConversationView/PipelineStageSection";

export interface PipelineStageState {
  stageIndex: number;
  stageName: string;
  agentLabel: string;
  status: StageStatus;
  text: string;
  startedAt: number | undefined;
  finishedAt: number | undefined;
}

function sortStagesByIndex(stages: PipelineStageState[]): PipelineStageState[] {
  return [...stages].sort((left, right) => left.stageIndex - right.stageIndex);
}

function mapStatus(s: string): StageStatus {
  switch (s) {
    case "running": return "running";
    case "completed": return "completed";
    case "failed": return "failed";
    case "stopped": return "stopped";
    default: return "pending";
  }
}

function isTerminalStatus(status: StageStatus): boolean {
  return status === "completed" || status === "failed" || status === "stopped";
}

/**
 * Earliest defined `startedAt` across the supplied stages.
 *
 * Derived from `stages` state so the value stays in lockstep with the
 * rendered stage list. Returning this as state (not a ref) prevents the
 * PipelineStatusBar from flickering out when the ref is reset mid-run
 * (e.g. during re-hydration via `loadFromSaved`).
 */
function earliestStageStart(stages: PipelineStageState[]): number | undefined {
  let earliest: number | undefined;
  for (const stage of stages) {
    if (stage.startedAt === undefined) continue;
    if (earliest === undefined || stage.startedAt < earliest) {
      earliest = stage.startedAt;
    }
  }
  return earliest;
}

export interface UsePipelineSessionReturn {
  stages: PipelineStageState[];
  debugLog: string;
  pipelineStartedAt: number | undefined;
  running: boolean;
  awaitingReview: boolean;
  currentStageName: string;
  userPrompt: string;
  /** Full reset — clears all stages and state. */
  reset: () => void;
  /** Soft reset — keeps existing stages but resets running/review flags. */
  softReset: () => void;
  loadDebugLog: (log: string) => void;
  loadFromSaved: (state: PipelineState, stillRunning?: boolean, conversationStatus?: ConversationStatus) => void;
}

/**
 * Manages live pipeline state and listens to Tauri events.
 *
 * @param activeConversationId — only events whose `conversationId` matches
 *   this value are processed. Pass `null` to ignore all events.
 */
export function usePipelineSession(
  activeConversationId: string | null,
): UsePipelineSessionReturn {
  const [stages, setStages] = useState<PipelineStageState[]>([]);
  const [debugLog, setDebugLog] = useState("");
  const [running, setRunning] = useState(false);
  const [awaitingReview, setAwaitingReview] = useState(false);
  const [currentStageName, setCurrentStageName] = useState("");
  const [userPrompt, setUserPrompt] = useState("");
  const runningNamesRef = useRef<Set<string>>(new Set());

  // Stable ref so event handlers always see the latest conversation ID
  // without needing to be recreated (which would re-register listeners).
  const conversationIdRef = useRef(activeConversationId);
  conversationIdRef.current = activeConversationId;

  // When the active conversation switches, clear the running-stage name
  // set so the new conversation doesn't inherit the previous
  // conversation's labels. The stage list itself is driven by
  // loadFromSaved, which owns the reset of visible state.
  useEffect(() => {
    runningNamesRef.current = new Set();
  }, [activeConversationId]);

  const handleStageStatus = useCallback((event: PipelineStageStatusEvent) => {
    if (event.conversationId !== conversationIdRef.current) return;

    const mappedStatus = mapStatus(event.status);
    const now = Date.now();

    if (mappedStatus === "running") {
      setRunning(true);
      setAwaitingReview(false);
    }

    // Track all running stage names.
    if (mappedStatus === "running") {
      runningNamesRef.current.add(event.stageName);
    } else {
      runningNamesRef.current.delete(event.stageName);
    }
    setCurrentStageName([...runningNamesRef.current].join(", "));

    setStages((prev) => {
      const copy = [...prev];
      const existingIndex = copy.findIndex((stage) => stage.stageIndex === event.stageIndex);
      const existing = existingIndex === -1
        ? {
          stageIndex: event.stageIndex,
          stageName: "",
          agentLabel: "",
          status: "pending" as const,
          text: "",
          startedAt: undefined,
          finishedAt: undefined,
        }
        : copy[existingIndex];
      // When re-emitting saved stages the event carries persisted timestamps.
      const persistedStart = event.startedAt
        ? new Date(event.startedAt).getTime()
        : undefined;
      const persistedFinish = event.finishedAt
        ? new Date(event.finishedAt).getTime()
        : undefined;
      const terminalWithoutFinish = isTerminalStatus(mappedStatus) && persistedFinish === undefined;
      const keepRunning = terminalWithoutFinish && existing.status === "running";

      const nextStage: PipelineStageState = {
        ...existing,
        stageIndex: event.stageIndex,
        stageName: event.stageName,
        agentLabel: event.agentLabel,
        status: keepRunning ? existing.status : mappedStatus,
        // When the backend sends plan file content with a completed status,
        // replace the accumulated SSE output so the user sees the actual plan.
        text: event.text ?? existing.text,
        startedAt: persistedStart
          ?? (mappedStatus === "running"
            ? (existing.status === "running" ? existing.startedAt ?? now : now)
            : existing.startedAt),
        finishedAt: keepRunning
          ? existing.finishedAt
          : persistedFinish
          ?? ((mappedStatus === "completed" || mappedStatus === "failed" || mappedStatus === "stopped")
            ? (existing.finishedAt ?? now)
            : undefined),
      };
      if (existingIndex === -1) {
        copy.push(nextStage);
      } else {
        copy[existingIndex] = nextStage;
      }
      return sortStagesByIndex(copy);
    });
  }, []);

  const handleStageDelta = useCallback((event: PipelineStageOutputDelta) => {
    if (event.conversationId !== conversationIdRef.current) return;

    setStages((prev) => {
      const copy = [...prev];
      const existingIndex = copy.findIndex((stage) => stage.stageIndex === event.stageIndex);
      const existing = existingIndex === -1
        ? {
          stageIndex: event.stageIndex,
          stageName: "",
          agentLabel: "",
          status: "pending" as const,
          text: "",
          startedAt: undefined,
          finishedAt: undefined,
        }
        : copy[existingIndex];
      const nextStage: PipelineStageState = {
        ...existing,
        text: existing.text ? `${existing.text}\n${event.text}` : event.text,
      };
      if (existingIndex === -1) {
        copy.push(nextStage);
      } else {
        copy[existingIndex] = nextStage;
      }
      return sortStagesByIndex(copy);
    });
  }, []);

  const handleDebugLog = useCallback((event: PipelineDebugLogEvent) => {
    if (event.conversationId !== conversationIdRef.current) return;
    setDebugLog((prev) => (prev ? `${prev}\n${event.line}` : event.line));
  }, []);

  // Listen to the overall conversation status so we can mark the pipeline as
  // no longer running once the backend reports a terminal state.
  const handleConversationStatus = useCallback((event: ConversationStatusEvent) => {
    if (event.conversation.id !== conversationIdRef.current) return;

    const status = event.conversation.status;
    if (status === "awaiting_review") {
      setRunning(false);
      setAwaitingReview(true);
      runningNamesRef.current.clear();
      setCurrentStageName("Review Plan");
    } else if (status === "completed" || status === "failed" || status === "stopped") {
      setRunning(false);
      setAwaitingReview(false);
      runningNamesRef.current.clear();
      setCurrentStageName(status === "stopped" ? "Stopped" : "");
    } else if (status === "running") {
      setRunning(true);
      setAwaitingReview(false);
    }
  }, []);

  useTauriEventListeners({
    listeners: [
      { event: PIPELINE_EVENTS.STAGE_STATUS, handler: handleStageStatus },
      { event: PIPELINE_EVENTS.STAGE_OUTPUT_DELTA, handler: handleStageDelta },
      { event: PIPELINE_EVENTS.DEBUG_LOG, handler: handleDebugLog },
      { event: CONVERSATION_EVENTS.STATUS, handler: handleConversationStatus },
    ],
  });

  const reset = useCallback(() => {
    setStages([]);
    setDebugLog("");
    setRunning(false);
    setAwaitingReview(false);
    setCurrentStageName("");
    setUserPrompt("");
    runningNamesRef.current.clear();
  }, []);

  /** Keep existing stages intact but reset running/review flags. */
  const softReset = useCallback(() => {
    setRunning(false);
    setAwaitingReview(false);
    setCurrentStageName("");
    runningNamesRef.current.clear();
  }, []);

  const loadDebugLog = useCallback((log: string) => {
    setDebugLog(log);
  }, []);

  const loadFromSaved = useCallback((
    state: PipelineState,
    stillRunning = false,
    conversationStatus?: ConversationStatus,
  ) => {
    setUserPrompt(state.userPrompt);
    const loaded: PipelineStageState[] = state.stages.map((s) => ({
      stageIndex: s.stageIndex,
      stageName: s.stageName,
      agentLabel: s.agentLabel,
      status: mapStatus(s.status),
      text: s.text,
      startedAt: s.startedAt ? new Date(s.startedAt).getTime() : undefined,
      finishedAt: s.finishedAt ? new Date(s.finishedAt).getTime() : undefined,
    }));
    setStages(sortStagesByIndex(loaded));
    setRunning(stillRunning);
    setAwaitingReview(conversationStatus === "awaiting_review");

    if (stillRunning) {
      const names = loaded.filter((s) => s.status === "running").map((s) => s.stageName);
      runningNamesRef.current = new Set(names);
      setCurrentStageName(names.join(", "));
    } else if (conversationStatus === "awaiting_review") {
      runningNamesRef.current.clear();
      setCurrentStageName("Review Plan");
    } else {
      runningNamesRef.current.clear();
      if (loaded.some((s) => s.status === "stopped")) {
        setCurrentStageName("Stopped");
        return;
      }
      const lastRunning = loaded.filter((s) => s.status === "running").pop();
      setCurrentStageName(lastRunning?.stageName ?? "");
    }
  }, []);

  // Derived from stages so the PipelineStatusBar guard tracks the rendered
  // stage list exactly. A ref-backed value could go stale mid-run and hide
  // the bar until the next stage-status event restored it.
  const pipelineStartedAt = useMemo(() => earliestStageStart(stages), [stages]);

  return {
    stages,
    debugLog,
    pipelineStartedAt,
    running,
    awaitingReview,
    currentStageName,
    userPrompt,
    reset,
    softReset,
    loadDebugLog,
    loadFromSaved,
  };
}
