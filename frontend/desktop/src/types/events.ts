import type { PipelineStage, StageStatus, JudgeVerdict } from "./pipeline";

/** Shared Tauri event names for pipeline runtime updates. */
export const PIPELINE_EVENTS = {
  started: "pipeline:started",
  stage: "pipeline:stage",
  log: "pipeline:log",
  artifact: "pipeline:artifact",
  question: "pipeline:question",
  completed: "pipeline:completed",
  error: "pipeline:error",
} as const;

// ---- Backend event payloads ----

/** Emitted when a new pipeline run begins. */
export interface PipelineStartedEvent {
  runId: string;
  sessionId: string;
  prompt: string;
  workspacePath: string;
}

/** Emitted when a pipeline stage changes status. */
export interface PipelineStageEvent {
  runId: string;
  stage: PipelineStage;
  status: StageStatus;
  iteration: number;
  durationMs?: number;
}

/** Emitted for each log line produced by an agent. */
export interface PipelineLogEvent {
  runId: string;
  stage: PipelineStage;
  line: string;
  stream: "stdout" | "stderr";
}

/** Emitted when a pipeline artefact is produced. */
export interface PipelineArtifactEvent {
  runId: string;
  kind: string;
  content: string;
  iteration: number;
}

/** Emitted when the pipeline completes successfully. */
export interface PipelineCompletedEvent {
  runId: string;
  verdict: JudgeVerdict;
  totalIterations: number;
  durationMs: number;
}

/** Emitted when the pipeline encounters an error. */
export interface PipelineErrorEvent {
  runId: string;
  stage?: PipelineStage;
  message: string;
}

/** Pipeline question event — emitted when the pipeline pauses for user input. */
export interface PipelineQuestionEvent {
  runId: string;
  questionId: string;
  stage: PipelineStage;
  iteration: number;
  questionText: string;
  agentOutput: string;
  optional: boolean;
}

/** Answer payload sent back to the backend via the answer_pipeline_question command. */
export interface PipelineAnswer {
  runId: string;
  questionId: string;
  answer: string;
  skipped: boolean;
}
