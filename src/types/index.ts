/** Agent role identifiers for the orchestration pipeline. */
export type AgentRole =
  | "generator"
  | "reviewer"
  | "fixer"
  | "validator"
  | "final_judge";

/** Supported CLI agent backends. */
export type AgentBackend = "claude" | "codex" | "gemini";

/** Pipeline stage identifiers. */
export type PipelineStage =
  | "generate"
  | "diff_after_generate"
  | "review"
  | "fix"
  | "diff_after_fix"
  | "validate"
  | "judge";

/** Status of a single pipeline stage. */
export type StageStatus = "pending" | "running" | "completed" | "failed" | "skipped";

/** Judge verdict — the final arbiter's decision. */
export type JudgeVerdict = "COMPLETE" | "NOT COMPLETE";

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

/** Overall pipeline run state. */
export type PipelineStatus =
  | "idle"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

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

/** Application settings persisted locally. */
export interface AppSettings {
  claudePath: string;
  codexPath: string;
  geminiPath: string;
  generatorAgent: AgentBackend;
  reviewerAgent: AgentBackend;
  fixerAgent: AgentBackend;
  validatorAgent: AgentBackend;
  finalJudgeAgent: AgentBackend;
  maxIterations: number;
  requireGit: boolean;
}

/** Default settings values. */
export const DEFAULT_SETTINGS: AppSettings = {
  claudePath: "claude",
  codexPath: "codex",
  geminiPath: "gemini",
  generatorAgent: "claude",
  reviewerAgent: "codex",
  fixerAgent: "claude",
  validatorAgent: "gemini",
  finalJudgeAgent: "codex",
  maxIterations: 3,
  requireGit: true,
};

/** CLI health check result returned from the backend. */
export interface CliHealth {
  claude: CliStatus;
  codex: CliStatus;
  gemini: CliStatus;
}

export interface CliStatus {
  available: boolean;
  path: string;
  error?: string;
}

/** Request to start a pipeline run. */
export interface PipelineRequest {
  prompt: string;
  workspacePath: string;
}

/** Workspace validation result. */
export interface WorkspaceInfo {
  path: string;
  isGitRepo: boolean;
  isDirty: boolean;
  branch?: string;
}

// ---- Backend event payloads ----

export interface PipelineStartedEvent {
  runId: string;
  prompt: string;
  workspacePath: string;
}

export interface PipelineStageEvent {
  runId: string;
  stage: PipelineStage;
  status: StageStatus;
  iteration: number;
}

export interface PipelineLogEvent {
  runId: string;
  stage: PipelineStage;
  line: string;
  stream: "stdout" | "stderr";
}

export interface PipelineArtifactEvent {
  runId: string;
  kind: "diff" | "review" | "validation" | "judge";
  content: string;
  iteration: number;
}

export interface PipelineCompletedEvent {
  runId: string;
  verdict: JudgeVerdict;
  totalIterations: number;
  durationMs: number;
}

export interface PipelineErrorEvent {
  runId: string;
  stage?: PipelineStage;
  message: string;
}
