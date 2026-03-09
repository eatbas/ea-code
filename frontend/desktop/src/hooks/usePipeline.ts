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
  cancelPipeline: () => Promise<void>;
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

  // Use ref to avoid stale closures in event handlers
  const runRef = useRef<PipelineRun | null>(null);
  runRef.current = run;

  useEffect(() => {
    const unlisteners: Promise<UnlistenFn>[] = [];

    // Pipeline started
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
        setRun(newRun);
        setLogs([]);
        setStageLogs({});
        setArtifacts({});
        setPendingQuestion(null);
      }),
    );

    // Stage status update
    unlisteners.push(
      listen<PipelineStageEvent>("pipeline:stage", (event) => {
        const { stage, status, iteration, durationMs } = event.payload;
        setRun((prev) => {
          if (!prev) return prev;
          const updated = { ...prev, currentStage: stage, currentIteration: iteration };

          // Update pipeline-level status for waiting_for_input
          if (status === "waiting_for_input") {
            updated.status = "waiting_for_input";
          }

          // Ensure the iteration array is long enough
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

    // Log line — add to both flat logs and per-stage logs
    unlisteners.push(
      listen<PipelineLogEvent>("pipeline:log", (event) => {
        const { stage, line, stream } = event.payload;
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

    // Artifact produced
    unlisteners.push(
      listen<PipelineArtifactEvent>("pipeline:artifact", (event) => {
        const { kind, content } = event.payload;
        setArtifacts((prev) => ({ ...prev, [kind]: content }));
      }),
    );

    // Pipeline question — pause for user input
    unlisteners.push(
      listen<PipelineQuestionEvent>("pipeline:question", (event) => {
        setPendingQuestion(event.payload);
      }),
    );

    // Pipeline completed
    unlisteners.push(
      listen<PipelineCompletedEvent>("pipeline:completed", (event) => {
        const { verdict, totalIterations, durationMs } = event.payload;
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
          };
        });
        setPendingQuestion(null);
      }),
    );

    // Pipeline error
    unlisteners.push(
      listen<PipelineErrorEvent>("pipeline:error", (event) => {
        const { message } = event.payload;
        setRun((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            status: "failed",
            error: message,
            completedAt: new Date().toISOString(),
            currentStage: undefined,
          };
        });
        setPendingQuestion(null);
      }),
    );

    // Clean up all listeners on unmount
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

  const cancelPipeline = useCallback(async (): Promise<void> => {
    try {
      await invoke("cancel_pipeline");
      setPendingQuestion(null);
      setRun((prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          status: "cancelled",
          completedAt: new Date().toISOString(),
          currentStage: undefined,
        };
      });
    } catch {
      toast.error("Failed to cancel pipeline.");
    }
  }, [toast]);

  const answerQuestion = useCallback(async (answer: PipelineAnswer): Promise<void> => {
    try {
      await invoke("answer_pipeline_question", { answer });
      setPendingQuestion(null);
      setRun((prev) => {
        if (!prev) return prev;
        return { ...prev, status: "running" };
      });
    } catch {
      // Don't clear the pending question so the user can retry
      toast.error("Failed to submit answer.");
    }
  }, [toast]);

  /** Clear all pipeline state and return to idle. */
  const resetRun = useCallback((): void => {
    setRun(null);
    setLogs([]);
    setStageLogs({});
    setArtifacts({});
    setPendingQuestion(null);
  }, []);

  return { run, logs, stageLogs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun };
}
