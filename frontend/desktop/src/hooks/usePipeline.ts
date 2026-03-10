import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useToast } from "../components/shared/Toast";
import type {
  PipelineRun,
  PipelineRequest,
  PipelineStartedEvent,
  PipelineStageEvent,
  PipelineLogEvent,
  PipelineArtifactEvent,
  PipelineCompletedEvent,
  PipelineErrorEvent,
  PipelineQuestionEvent,
  PipelineAnswer,
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
  function isCurrentRunEvent(runId: string): boolean {
    return runRef.current?.id === runId;
  }

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen<PipelineStartedEvent>("pipeline:started", (event) => {
        const payload = event.payload;
        const newRun: PipelineRun = {
          id: payload.runId,
          sessionId: payload.sessionId,
          status: "running",
          prompt: payload.prompt,
          workspacePath: payload.workspacePath,
          iterations: [],
          currentIteration: 1,
          maxIterations: 3,
          startedAt: new Date().toISOString(),
        };
        runRef.current = newRun;
        setRun(newRun);
        setLogs([]);
        setStageLogs({});
        setArtifacts({});
        setPendingQuestion(null);
      }),
    );

    unlisteners.push(
      listen<PipelineStageEvent>("pipeline:stage", (event) => {
        const { runId, stage, status, iteration, durationMs } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        setRun((prev) => {
          if (!prev) return prev;
          const updated = { ...prev, currentStage: stage, currentIteration: iteration, stageStartedAt: Date.now() };

          if (status === "waiting_for_input") {
            updated.status = "waiting_for_input";
          }

          const iterations = [...updated.iterations];
          while (iterations.length < iteration) {
            iterations.push({ number: iterations.length + 1, stages: [] });
          }

          const currentIter = { ...iterations[iteration - 1] };
          const stages = [...currentIter.stages];
          const existingIdx = stages.findIndex((s) => s.stage === stage);

          const stageResult = {
            stage,
            status,
            output: "",
            durationMs: durationMs ?? 0,
          };

          if (existingIdx >= 0) {
            stages[existingIdx] = { ...stages[existingIdx], status, durationMs: durationMs ?? stages[existingIdx].durationMs };
          } else {
            stages.push(stageResult);
          }

          currentIter.stages = stages;
          iterations[iteration - 1] = currentIter;
          updated.iterations = iterations;

          return updated;
        });
      }),
    );

    unlisteners.push(
      listen<PipelineLogEvent>("pipeline:log", (event) => {
        const { runId, stage, line, stream } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        const prefix = stream === "stderr" ? "[stderr] " : "";
        const formatted = `${prefix}${line}`;
        setLogs((prev) => [...prev, formatted]);
        setStageLogs((prev) => {
          const key = stage as string;
          const existing = prev[key] ?? [];
          return { ...prev, [key]: [...existing, formatted] };
        });
      }),
    );

    unlisteners.push(
      listen<PipelineArtifactEvent>("pipeline:artifact", (event) => {
        const { runId, kind, content } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        setArtifacts((prev) => ({ ...prev, [kind]: content }));
      }),
    );

    unlisteners.push(
      listen<PipelineQuestionEvent>("pipeline:question", (event) => {
        if (!isCurrentRunEvent(event.payload.runId)) return;
        setPendingQuestion(event.payload);
      }),
    );

    unlisteners.push(
      listen<PipelineCompletedEvent>("pipeline:completed", (event) => {
        const { runId, verdict, totalIterations, durationMs } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        setRun((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            status: "completed",
            finalVerdict: verdict,
            currentIteration: totalIterations,
            durationMs,
            completedAt: new Date().toISOString(),
            currentStage: undefined,
            stageStartedAt: undefined,
          };
        });
        setPendingQuestion(null);
      }),
    );

    unlisteners.push(
      listen<PipelineErrorEvent>("pipeline:error", (event) => {
        const { runId, message } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        const lowerMessage = message.toLowerCase();
        const nextStatus = lowerMessage.includes("cancel") ? "cancelled" : "failed";
        setRun((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            status: nextStatus,
            error: message,
            completedAt: new Date().toISOString(),
            currentStage: undefined,
            stageStartedAt: undefined,
          };
        });
        setPendingQuestion(null);
      }),
    );

    return () => {
      unlisteners.forEach((promise) => {
        promise.then((unlisten) => unlisten());
      });
    };
  }, []);

  const startPipeline = useCallback(async (request: PipelineRequest): Promise<void> => {
    try {
      await invoke("run_pipeline", { request });
    } catch (err) {
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
      setRun((prev) => {
        if (!prev || prev.id !== targetRunId) return prev;
        return { ...prev, status: "paused", stageStartedAt: undefined };
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
