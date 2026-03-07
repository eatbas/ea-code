import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  PipelineRun,
  PipelineRequest,
  PipelineStartedEvent,
  PipelineStageEvent,
  PipelineLogEvent,
  PipelineArtifactEvent,
  PipelineCompletedEvent,
  PipelineErrorEvent,
} from "../types";

interface UsePipelineReturn {
  run: PipelineRun | null;
  logs: string[];
  artifacts: Record<string, string>;
  startPipeline: (request: PipelineRequest) => Promise<void>;
  cancelPipeline: () => Promise<void>;
}

/** Hook managing the full pipeline lifecycle including Tauri event listeners. */
export function usePipeline(): UsePipelineReturn {
  const [run, setRun] = useState<PipelineRun | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [artifacts, setArtifacts] = useState<Record<string, string>>({});

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
        setArtifacts({});
      }),
    );

    // Stage status update
    unlisteners.push(
      listen<PipelineStageEvent>("pipeline:stage", (event) => {
        const { stage, status, iteration } = event.payload;
        setRun((prev) => {
          if (!prev) return prev;
          const updated = { ...prev, currentStage: stage, currentIteration: iteration };

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
            durationMs: 0,
          };

          if (existingIdx >= 0) {
            stages[existingIdx] = { ...stages[existingIdx], status };
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

    // Log line
    unlisteners.push(
      listen<PipelineLogEvent>("pipeline:log", (event) => {
        const { line, stream } = event.payload;
        const prefix = stream === "stderr" ? "[stderr] " : "";
        setLogs((prev) => [...prev, `${prefix}${line}`]);
      }),
    );

    // Artifact produced
    unlisteners.push(
      listen<PipelineArtifactEvent>("pipeline:artifact", (event) => {
        const { kind, content } = event.payload;
        setArtifacts((prev) => ({ ...prev, [kind]: content }));
      }),
    );

    // Pipeline completed
    unlisteners.push(
      listen<PipelineCompletedEvent>("pipeline:completed", (event) => {
        const { verdict, totalIterations } = event.payload;
        setRun((prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            status: "completed",
            finalVerdict: verdict,
            currentIteration: totalIterations,
            completedAt: new Date().toISOString(),
            currentStage: undefined,
          };
        });
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
    await invoke("run_pipeline", { request });
  }, []);

  const cancelPipeline = useCallback(async (): Promise<void> => {
    await invoke("cancel_pipeline");
    setRun((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        status: "cancelled",
        completedAt: new Date().toISOString(),
        currentStage: undefined,
      };
    });
  }, []);

  return { run, logs, artifacts, startPipeline, cancelPipeline };
}
