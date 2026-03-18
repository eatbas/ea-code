/** Pipeline stage identifiers. */
export type PipelineStage =
  | "prompt_enhance"
  | "skill_select"
  | "plan"
  | "plan2"
  | "plan3"
  | "plan_audit"
  | "coder"
  | "code_reviewer"
  | "code_reviewer2"
  | "code_reviewer3"
  | "review_merge"
  | "code_fixer"
  | "judge"
  | "executive_summary"
  | "direct_task";

/** Status of a single pipeline stage. */
export type StageStatus = "pending" | "running" | "completed" | "failed" | "skipped" | "waiting_for_input";

/** Judge verdict — the final arbiter's decision. */
export type JudgeVerdict = "COMPLETE" | "NOT COMPLETE";

/** Overall pipeline run state. */
export type PipelineStatus =
  | "idle"
  | "running"
  | "paused"
  | "waiting_for_input"
  | "completed"
  | "failed"
  | "cancelled";

/** Represents one stage's result in the pipeline timeline.
 *  Note: This is used for live pipeline UI, not for stored data.
 */
export interface StageResult {
  stage: PipelineStage;
  status: StageStatus;
  output: string;
  durationMs: number;
  /** Absolute timestamp (Date.now()) when this stage entered a running state. */
  startedAt?: number;
  error?: string;
}

/** A single iteration of the self-improving loop.
 *  Note: This is used for live pipeline UI, not for stored data.
 */
export interface Iteration {
  number: number;
  stages: StageResult[];
  verdict?: JudgeVerdict;
  judgeReasoning?: string;
}

/** Full pipeline run state for the frontend.
 *  Note: This is used for live pipeline UI, not for stored data.
 */
export interface PipelineRun {
  id: string;
  sessionId?: string;
  status: PipelineStatus;
  prompt: string;
  workspacePath: string;
  iterations: Iteration[];
  currentIteration: number;
  currentStage?: PipelineStage;
  /** Absolute timestamp (Date.now()) when the current stage started — used for persistent timer. */
  stageStartedAt?: number;
  maxIterations: number;
  startedAt?: string;
  completedAt?: string;
  durationMs?: number;
  finalVerdict?: JudgeVerdict;
  error?: string;
}
