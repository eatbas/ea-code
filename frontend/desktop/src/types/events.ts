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
  maxIterations: number;
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

// ---- JSONL Event Log Types ----

/** Status values for stage completion in JSONL events. */
export type StageEndStatus = "completed" | "failed" | "skipped";

/** Run status values for JSONL events and run summaries. */
export type RunStatus =
  | "running"
  | "paused"
  | "waiting_for_input"
  | "completed"
  | "failed"
  | "cancelled"
  | "crashed";

/** Union type for all events stored in events.jsonl.
 *
 * Every event has schema version (v), sequence number (seq), and
 * timestamp (ts) in RFC 3339 format. The discriminated union uses
 * the `type` field to distinguish variants.
 */
export type RunEvent =
  | RunStartEvent
  | StageStartEvent
  | StageEndEvent
  | IterationEndEvent
  | QuestionEvent
  | RunEndEvent;

/** Base fields present in all run events. */
export interface RunEventBase {
  /** Schema version for migration handling. */
  v: number;
  /** Monotonic sequence number (1-based). */
  seq: number;
  /** Timestamp in RFC 3339 format. */
  ts: string;
}

/** Marks the beginning of a pipeline run. */
export interface RunStartEvent extends RunEventBase {
  type: "run_start";
  /** The original user prompt. */
  prompt: string;
  /** Maximum iterations allowed for this run. */
  maxIterations: number;
}

/** A stage begins execution. */
export interface StageStartEvent extends RunEventBase {
  type: "stage_start";
  /** The stage being started. */
  stage: PipelineStage;
  /** Current iteration number (1-based). */
  iteration: number;
}

/** A stage finishes execution. */
export interface StageEndEvent extends RunEventBase {
  type: "stage_end";
  /** The stage that finished. */
  stage: PipelineStage;
  /** Current iteration number (1-based). */
  iteration: number;
  /** Completion status of the stage. */
  status: StageEndStatus;
  /** Duration in milliseconds. */
  durationMs: number;
  /** Judge verdict (only for judge stage). */
  verdict?: JudgeVerdict;
  /** Plan audit verdict (only for plan_audit stage). */
  auditVerdict?: "APPROVED" | "REJECTED" | "NEEDS_REVISION";
}

/** An iteration loop completes with judge verdict. */
export interface IterationEndEvent extends RunEventBase {
  type: "iteration_end";
  /** The iteration that just completed (1-based). */
  iteration: number;
  /** Judge verdict for this iteration. */
  verdict: JudgeVerdict;
}

/** User answered a question during the run. */
export interface QuestionEvent extends RunEventBase {
  type: "question";
  /** The stage that asked the question. */
  stage: PipelineStage;
  /** Current iteration when question was asked (1-based). */
  iteration: number;
  /** The question text. */
  question: string;
  /** The user's answer (may be empty if skipped). */
  answer: string;
  /** Whether the question was skipped. */
  skipped: boolean;
}

/** Terminal event — run completed, failed, or cancelled.
 *
 * Every run MUST end with exactly one RunEndEvent.
 * If no terminal event exists, the run is considered crashed.
 */
export interface RunEndEvent extends RunEventBase {
  type: "run_end";
  /** Final run status. */
  status: RunStatus;
  /** Final judge verdict (null if failed/cancelled before judge). */
  verdict: JudgeVerdict | null;
  /** Error message if status is failed. */
  error?: string;
  /** Timestamp when the crash was recovered (if recovered). */
  recoveredAt?: string;
}
