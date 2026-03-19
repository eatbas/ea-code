import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "../components/shared/Toast";
import { isPlanStage } from "../components/shared/TabbedPlanCard";
import { isReviewStage } from "../components/shared/TabbedReviewCard";
import { usePipelineEvents } from "./usePipelineEvents";
import type {
  PipelineRun,
  PipelineRequest,
  PipelineQuestionEvent,
  PipelineAnswer,
  PipelineStage,
} from "../types";

interface UsePipelineReturn {
  run: PipelineRun | null;
  logs: string[];
  stageLogs: Record<string, string[]>;
  artifacts: Record<string, string>;
  pendingQuestion: PipelineQuestionEvent | null;
  startPipeline: (request: PipelineRequest) => Promise<void>;
  pausePipeline: (runId?: string) => Promise<void>;
  resumePipeline: (runId?: string) => Promise<void>;
  cancelPipeline: (runId?: string) => Promise<void>;
  answerQuestion: (answer: PipelineAnswer) => Promise<void>;
  resetRun: () => void;
}

/** Returns a predicate matching stages that should be removed when the given stage is paused. */
function stageResetPredicate(stage: PipelineStage | undefined): (s: PipelineStage) => boolean {
  if (!stage) return () => false;
  if (isPlanStage(stage)) return isPlanStage;
  if (isReviewStage(stage)) return isReviewStage;
  return (s) => s === stage;
}

function pruneArtifactsForPausedStage(
  artifacts: Record<string, string>,
  stage: PipelineStage | undefined,
): Record<string, string> {
  if (!stage) return artifacts;

  const next = { ...artifacts };
  if (isPlanStage(stage)) {
    for (const key of Object.keys(next)) {
      if (key === "plan" || /^plan_\d+$/.test(key)) {
        delete next[key];
      }
    }
    return next;
  }
  if (isReviewStage(stage)) {
    for (const key of Object.keys(next)) {
      if (key === "review" || /^review_\d+$/.test(key)) {
        delete next[key];
      }
    }
    return next;
  }
  return next;
}

/** Hook managing the full pipeline lifecycle including Tauri event listeners. */
export function usePipeline(): UsePipelineReturn {
  const toast = useToast();
  const [run, setRun] = useState<PipelineRun | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [stageLogs, setStageLogs] = useState<Record<string, string[]>>({});
  const [artifacts, setArtifacts] = useState<Record<string, string>>({});
  const [pendingQuestion, setPendingQuestion] = useState<PipelineQuestionEvent | null>(null);

  const runRef = useRef<PipelineRun | null>(null);
  runRef.current = run;

  const expectingStartRef = useRef(false);

  usePipelineEvents({ runRef, expectingStartRef, setRun, setLogs, setStageLogs, setArtifacts, setPendingQuestion });

  const startPipeline = useCallback(async (request: PipelineRequest): Promise<void> => {
    try {
      expectingStartRef.current = true;
      await invoke("run_pipeline", { request });
    } catch (err) {
      expectingStartRef.current = false;
      const message = err instanceof Error ? err.message : String(err);
      setRun((prev) => {
        if (!prev) return prev;
        return { ...prev, status: "failed", error: message, currentStage: undefined };
      });
      toast.error("Failed to start pipeline.");
    }
  }, [toast]);

  const pausePipeline = useCallback(async (runId?: string): Promise<void> => {
    const targetRunId = typeof runId === "string" && runId.length > 0 ? runId : runRef.current?.id;
    if (!targetRunId) return;
    try {
      await invoke("pause_pipeline", { runId: targetRunId });
      const activeStage = runRef.current?.currentStage;
      const shouldReset = stageResetPredicate(activeStage);
      setArtifacts((prev) => pruneArtifactsForPausedStage(prev, activeStage));
      setRun((prev) => {
        if (!prev || prev.id !== targetRunId) return prev;
        const currentIterationIndex = Math.max(0, prev.currentIteration - 1);
        const iterations = prev.iterations.map((iteration, index) => {
          if (index !== currentIterationIndex) return iteration;
          return {
            ...iteration,
            stages: iteration.stages.filter((entry) => !shouldReset(entry.stage)),
          };
        });
        return { ...prev, status: "paused", stageStartedAt: undefined, iterations };
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Failed to pause pipeline: ${message}`);
    }
  }, [toast]);

  const resumePipeline = useCallback(async (runId?: string): Promise<void> => {
    const targetRunId = typeof runId === "string" && runId.length > 0 ? runId : runRef.current?.id;
    if (!targetRunId) return;

    try {
      await invoke("resume_pipeline", { runId: targetRunId });
      setRun((prev) => {
        if (!prev || prev.id !== targetRunId) return prev;
        return { ...prev, status: "running", stageStartedAt: Date.now() };
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Failed to resume pipeline: ${message}`);
    }
  }, [toast]);

  const cancelPipeline = useCallback(async (runId?: string): Promise<void> => {
    const targetRunId = typeof runId === "string" && runId.length > 0 ? runId : runRef.current?.id;
    if (!targetRunId) return;

    try {
      await invoke("cancel_pipeline", { runId: targetRunId });
      setPendingQuestion(null);
      setRun((prev) => {
        if (!prev || prev.id !== targetRunId) return prev;
        return {
          ...prev,
          status: "cancelled",
          completedAt: new Date().toISOString(),
          currentStage: undefined,
          stageStartedAt: undefined,
        };
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Failed to cancel pipeline: ${message}`);
    }
  }, [toast]);

  const answerQuestion = useCallback(async (answer: PipelineAnswer): Promise<void> => {
    try {
      await invoke("answer_pipeline_question", { answer });
      setPendingQuestion(null);
      setRun((prev) => {
        if (!prev || prev.id !== answer.runId) return prev;
        const nextStatus = prev.status === "paused" ? "paused" : "running";
        return { ...prev, status: nextStatus };
      });
    } catch {
      toast.error("Failed to submit answer.");
    }
  }, [toast]);

  /** Clear all pipeline state and return to idle. */
  const resetRun = useCallback((): void => {
    runRef.current = null;
    setRun(null);
    setLogs([]);
    setStageLogs({});
    setArtifacts({});
    setPendingQuestion(null);
  }, []);

  return {
    run,
    logs,
    stageLogs,
    artifacts,
    pendingQuestion,
    startPipeline,
    pausePipeline,
    resumePipeline,
    cancelPipeline,
    answerQuestion,
    resetRun,
  };
}
