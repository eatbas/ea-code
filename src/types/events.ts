import type { PipelineStage, StageStatus, JudgeVerdict } from "./pipeline";

// ---- Backend event payloads ----

/** Emitted when a new pipeline run begins. */
export interface PipelineStartedEvent {
  runId: string;
  prompt: string;
  workspacePath: string;
}

/** Emitted when a pipeline stage changes status. */
export interface PipelineStageEvent {
  runId: string;
  stage: PipelineStage;
  status: StageStatus;
  iteration: number;
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
  kind: "diff" | "plan" | "plan_audit" | "plan_final" | "review" | "judge" | "executive_summary";
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
  questionId: string;
  answer: string;
  skipped: boolean;
}
