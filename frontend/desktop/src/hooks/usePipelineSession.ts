import { useCallback, useRef, useState } from "react";
import type {
  ConversationStatusEvent,
  PipelineState,
  PipelineStageStatusEvent,
  PipelineStageOutputDelta,
} from "../types";
import { CONVERSATION_EVENTS, PIPELINE_EVENTS } from "../constants/events";
import { useTauriEventListeners } from "./useTauriEventListeners";
import type { StageStatus } from "../components/ConversationView/PipelineStageSection";

export interface PipelineStageState {
  stageName: string;
  agentLabel: string;
  status: StageStatus;
  text: string;
  startedAt: number | undefined;
  finishedAt: number | undefined;
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

export interface UsePipelineSessionReturn {
  stages: PipelineStageState[];
  pipelineStartedAt: number | undefined;
  running: boolean;
  currentStageName: string;
  userPrompt: string;
  reset: () => void;
  loadFromSaved: (state: PipelineState, stillRunning?: boolean) => void;
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
  const [running, setRunning] = useState(false);
  const [currentStageName, setCurrentStageName] = useState("");
  const [userPrompt, setUserPrompt] = useState("");
  const pipelineStartRef = useRef<number | undefined>(undefined);
  const runningNamesRef = useRef<Set<string>>(new Set());

  // Stable ref so event handlers always see the latest conversation ID
  // without needing to be recreated (which would re-register listeners).
  const conversationIdRef = useRef(activeConversationId);
  conversationIdRef.current = activeConversationId;

  const handleStageStatus = useCallback((event: PipelineStageStatusEvent) => {
    if (event.conversationId !== conversationIdRef.current) return;

    const mappedStatus = mapStatus(event.status);
    const now = Date.now();

    if (mappedStatus === "running" && !pipelineStartRef.current) {
      pipelineStartRef.current = now;
      setRunning(true);
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
      while (copy.length <= event.stageIndex) {
        copy.push({
          stageName: "",
          agentLabel: "",
          status: "pending",
          text: "",
          startedAt: undefined,
          finishedAt: undefined,
        });
      }
      const existing = copy[event.stageIndex];
      copy[event.stageIndex] = {
        stageName: event.stageName,
        agentLabel: event.agentLabel,
        status: mappedStatus,
        text: existing.text,
        startedAt: existing.startedAt ?? (mappedStatus === "running" ? now : undefined),
        finishedAt: mappedStatus === "completed" || mappedStatus === "failed" ? now : undefined,
      };
      return copy;
    });
  }, []);

  const handleStageDelta = useCallback((event: PipelineStageOutputDelta) => {
    if (event.conversationId !== conversationIdRef.current) return;

    setStages((prev) => {
      const copy = [...prev];
      if (event.stageIndex < copy.length) {
        const existing = copy[event.stageIndex];
        copy[event.stageIndex] = {
          ...existing,
          text: existing.text ? `${existing.text}\n${event.text}` : event.text,
        };
      }
      return copy;
    });
  }, []);

  // Listen to the overall conversation status so we can mark the pipeline as
  // no longer running once the backend reports a terminal state.
  const handleConversationStatus = useCallback((event: ConversationStatusEvent) => {
    if (event.conversation.id !== conversationIdRef.current) return;

    const status = event.conversation.status;
    if (status === "completed" || status === "failed" || status === "stopped") {
      setRunning(false);
      runningNamesRef.current.clear();
      setCurrentStageName(status === "stopped" ? "Stopped" : "");
    }
  }, []);

  useTauriEventListeners({
    listeners: [
      { event: PIPELINE_EVENTS.STAGE_STATUS, handler: handleStageStatus },
      { event: PIPELINE_EVENTS.STAGE_OUTPUT_DELTA, handler: handleStageDelta },
      { event: CONVERSATION_EVENTS.STATUS, handler: handleConversationStatus },
    ],
  });

  const reset = useCallback(() => {
    setStages([]);
    setRunning(false);
    setCurrentStageName("");
    setUserPrompt("");
    pipelineStartRef.current = undefined;
    runningNamesRef.current.clear();
  }, []);

  const loadFromSaved = useCallback((state: PipelineState, stillRunning = false) => {
    setUserPrompt(state.userPrompt);
    const loaded: PipelineStageState[] = state.stages.map((s) => ({
      stageName: s.stageName,
      agentLabel: s.agentLabel,
      status: mapStatus(s.status),
      text: s.text,
      startedAt: s.startedAt ? new Date(s.startedAt).getTime() : undefined,
      finishedAt: s.finishedAt ? new Date(s.finishedAt).getTime() : undefined,
    }));
    setStages(loaded);
    setRunning(stillRunning);

    const firstStarted = loaded
      .map((s) => s.startedAt)
      .filter((t): t is number => t !== undefined)
      .sort()[0];
    pipelineStartRef.current = firstStarted;

    if (stillRunning) {
      const names = loaded.filter((s) => s.status === "running").map((s) => s.stageName);
      runningNamesRef.current = new Set(names);
      setCurrentStageName(names.join(", "));
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

  return {
    stages,
    pipelineStartedAt: pipelineStartRef.current,
    running,
    currentStageName,
    userPrompt,
    reset,
    loadFromSaved,
  };
}
