/** Pipeline stage identifiers. */
export type PipelineStage =
  | "prompt_enhance"
  | "plan"
  | "plan_audit"
  | "generate"
  | "diff_after_generate"
  | "review"
  | "fix"
  | "diff_after_fix"
  | "judge"
  | "executive_summary";

/** Status of a single pipeline stage. */
export type StageStatus = "pending" | "running" | "completed" | "failed" | "skipped" | "waiting_for_input";

/** Judge verdict — the final arbiter's decision. */
export type JudgeVerdict = "COMPLETE" | "NOT COMPLETE";

/** Overall pipeline run state. */
export type PipelineStatus =
  | "idle"
  | "running"
  | "waiting_for_input"
  | "completed"
  | "failed"
  | "cancelled";

/** Represents one stage's result in the pipeline timeline. */
export interface StageResult {
  stage: PipelineStage;
  status: StageStatus;
  output: string;
  durationMs: number;
  error?: string;
}

/** A single iteration of the self-improving loop. */
export interface Iteration {
  number: number;
  stages: StageResult[];
  verdict?: JudgeVerdict;
  judgeReasoning?: string;
}

/** Full pipeline run state for the frontend. */
export interface PipelineRun {
  id: string;
  status: PipelineStatus;
  prompt: string;
  workspacePath: string;
  iterations: Iteration[];
  currentIteration: number;
  currentStage?: PipelineStage;
  maxIterations: number;
  startedAt?: string;
  completedAt?: string;
  finalVerdict?: JudgeVerdict;
  error?: string;
}
