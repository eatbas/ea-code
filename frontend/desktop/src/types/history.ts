import type { AgentBackend } from "./agents";

/** Options passed from IdleView to App when submitting a prompt. */
export interface RunOptions {
  prompt: string;
  directTask: boolean;
  directTaskAgent?: AgentBackend;
  directTaskModel?: string;
  noPlan: boolean;
}

/** Request to start a pipeline run. */
export interface PipelineRequest {
  prompt: string;
  workspacePath: string;
  /** Session ID for this conversation thread. If omitted, a new session is created. */
  sessionId?: string;
  /** When true, bypass the pipeline and send the prompt directly to a single agent. */
  directTask?: boolean;
  /** Agent backend to use for direct task mode. */
  directTaskAgent?: AgentBackend;
  /** Model to use for direct task mode. */
  directTaskModel?: string;
  /** When true, skip the plan and plan_audit stages in the pipeline. */
  noPlan?: boolean;
}

/** Workspace validation result. */
export interface WorkspaceInfo {
  path: string;
  isGitRepo: boolean;
  branch?: string;
}

// ---- History / persistence types ----

/** Project bookmark stored in the database. */
export interface ProjectSummary {
  id: number;
  path: string;
  name: string;
  isGitRepo: boolean;
  branch?: string;
  lastOpened: string;
  createdAt: string;
}

/** Lightweight session summary for the sidebar. */
export interface SessionSummary {
  id: string;
  title: string;
  projectId: number;
  runCount: number;
  lastPrompt?: string;
  lastStatus?: string;
  createdAt: string;
  updatedAt: string;
}

/** Full session detail with paginated runs. */
export interface SessionDetail {
  id: string;
  title: string;
  projectPath: string;
  createdAt: string;
  updatedAt: string;
  runs: RunDetail[];
  /** Total number of runs in this session (for pagination). */
  totalRuns: number;
}

/** Lightweight run summary for history lists. */
export interface RunSummary {
  id: string;
  prompt: string;
  status: string;
  finalVerdict?: string;
  executiveSummary?: string;
  startedAt: string;
  completedAt?: string;
}

/** Full run detail with iterations, stages, and questions. */
export interface RunDetail {
  id: string;
  prompt: string;
  status: string;
  finalVerdict?: string;
  error?: string;
  executiveSummary?: string;
  executiveSummaryStatus?: string;
  executiveSummaryError?: string;
  executiveSummaryAgent?: string;
  executiveSummaryModel?: string;
  executiveSummaryGeneratedAt?: string;
  maxIterations: number;
  startedAt: string;
  completedAt?: string;
  currentStage?: string;
  currentIteration: number;
  currentStageStartedAt?: string;
  iterations: IterationDetail[];
  questions: QuestionEntry[];
}

/** Full iteration detail with stages. */
export interface IterationDetail {
  number: number;
  verdict?: string;
  judgeReasoning?: string;
  stages: StageEntry[];
}

/** Stored stage entry from the database. */
export interface StageEntry {
  id: number;
  iterationId: number;
  stage: string;
  status: string;
  output: string;
  durationMs: number;
  error?: string;
  createdAt: string;
}

/** Stored artefact entry from the database. */
export interface ArtifactEntry {
  id: number;
  runId: string;
  iteration: number;
  kind: string;
  content: string;
  createdAt: string;
}

/** Stored question/answer entry from the database. */
export interface QuestionEntry {
  id: string;
  runId: string;
  stage: string;
  iteration: number;
  questionText: string;
  agentOutput: string;
  optional: boolean;
  answer?: string;
  skipped: boolean;
  askedAt: string;
  answeredAt?: string;
}
