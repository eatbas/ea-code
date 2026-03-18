import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { PIPELINE_EVENTS } from "../types";
import type {
  PipelineRun,
  PipelineStartedEvent,
  PipelineStageEvent,
  PipelineLogEvent,
  PipelineArtifactEvent,
  PipelineCompletedEvent,
  PipelineErrorEvent,
  PipelineQuestionEvent,
} from "../types";

interface PipelineStateSetters {
  runRef: MutableRefObject<PipelineRun | null>;
  setRun: Dispatch<SetStateAction<PipelineRun | null>>;
  setLogs: Dispatch<SetStateAction<string[]>>;
  setStageLogs: Dispatch<SetStateAction<Record<string, string[]>>>;
  setArtifacts: Dispatch<SetStateAction<Record<string, string>>>;
  setPendingQuestion: Dispatch<SetStateAction<PipelineQuestionEvent | null>>;
}

/** Subscribes to all Tauri pipeline events and updates the provided state. */
export function usePipelineEvents({
  runRef,
  setRun,
  setLogs,
  setStageLogs,
  setArtifacts,
  setPendingQuestion,
}: PipelineStateSetters): void {
  useEffect(() => {
    function isCurrentRunEvent(runId: string): boolean {
      return runRef.current?.id === runId;
    }

    const unlisteners: Promise<UnlistenFn>[] = [];

    unlisteners.push(
      listen<PipelineStartedEvent>(PIPELINE_EVENTS.started, (event) => {
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
      listen<PipelineStageEvent>(PIPELINE_EVENTS.stage, (event) => {
        const { runId, stage, status, iteration, durationMs } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        const isActiveStageStatus = status === "running" || status === "waiting_for_input";
        if (status === "running") {
          const stageLabel = stage.replace(/_/g, " ");
          const stageLine = `[system] Stage started: ${stageLabel} (iteration ${iteration})`;
          setLogs((prev) => [...prev, stageLine]);
          setStageLogs((prev) => {
            const key = stage as string;
            const existing = prev[key] ?? [];
            if (existing[existing.length - 1] === stageLine) return prev;
            return { ...prev, [key]: [...existing, stageLine] };
          });
        }
        setRun((prev) => {
          if (!prev) return prev;

          const iterations = [...prev.iterations];
          while (iterations.length < iteration) {
            iterations.push({ number: iterations.length + 1, stages: [] });
          }

          const currentIter = { ...iterations[iteration - 1] };
          const stages = [...currentIter.stages];
          const existingIdx = stages.findIndex((s) => s.stage === stage);
          const existingStage = existingIdx >= 0 ? stages[existingIdx] : undefined;
          const startedAt = isActiveStageStatus
            ? existingStage?.startedAt ?? Date.now()
            : existingStage?.startedAt;

          const stageResult = {
            stage,
            status,
            output: existingStage?.output ?? "",
            durationMs: durationMs ?? existingStage?.durationMs ?? 0,
            startedAt,
          };

          if (existingIdx >= 0) {
            stages[existingIdx] = {
              ...stages[existingIdx],
              status,
              durationMs: durationMs ?? stages[existingIdx].durationMs,
              startedAt,
            };
          } else {
            stages.push(stageResult);
          }

          currentIter.stages = stages;
          iterations[iteration - 1] = currentIter;

          const latestActiveStage = [...stages]
            .reverse()
            .find((entry) => entry.status === "running" || entry.status === "waiting_for_input");

          const updated: PipelineRun = {
            ...prev,
            currentIteration: iteration,
            currentStage: latestActiveStage?.stage,
            stageStartedAt: latestActiveStage?.startedAt,
            iterations,
          };

          if (status === "waiting_for_input") {
            updated.status = "waiting_for_input";
          } else if (status === "running" && prev.status !== "running") {
            updated.status = "running";
          }

          return updated;
        });
      }),
    );

    unlisteners.push(
      listen<PipelineLogEvent>(PIPELINE_EVENTS.log, (event) => {
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
      listen<PipelineArtifactEvent>(PIPELINE_EVENTS.artifact, (event) => {
        const { runId, kind, content } = event.payload;
        if (!isCurrentRunEvent(runId)) return;
        setArtifacts((prev) => ({ ...prev, [kind]: content }));
      }),
    );

    unlisteners.push(
      listen<PipelineQuestionEvent>(PIPELINE_EVENTS.question, (event) => {
        if (!isCurrentRunEvent(event.payload.runId)) return;
        setPendingQuestion(event.payload);
      }),
    );

    unlisteners.push(
      listen<PipelineCompletedEvent>(PIPELINE_EVENTS.completed, (event) => {
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
      listen<PipelineErrorEvent>(PIPELINE_EVENTS.error, (event) => {
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
  }, [runRef, setRun, setLogs, setStageLogs, setArtifacts, setPendingQuestion]);
}
