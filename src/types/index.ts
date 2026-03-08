/** Agent role identifiers for the orchestration pipeline. */
export type AgentRole =
  | "prompt_enhancer"
  | "planner"
  | "plan_auditor"
  | "coder"
  | "reviewer_auditor"
  | "code_fixer"
  | "judge"
  | "executive_summary";

/** Supported CLI agent backends. */
export type AgentBackend = "claude" | "codex" | "gemini" | "kimi" | "opencode";

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
  | "waiting_for_input"
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
  kimiPath: string;
  opencodePath: string;
  promptEnhancerAgent: AgentBackend;
  plannerAgent: AgentBackend | null;
  planAuditorAgent: AgentBackend | null;
  generatorAgent: AgentBackend;
  reviewerAgent: AgentBackend;
  fixerAgent: AgentBackend;
  finalJudgeAgent: AgentBackend;
  executiveSummaryAgent: AgentBackend;
  maxIterations: number;
  requireGit: boolean;
  /** Comma-separated list of enabled Claude models. */
  claudeModel: string;
  /** Comma-separated list of enabled Codex models. */
  codexModel: string;
  /** Comma-separated list of enabled Gemini models. */
  geminiModel: string;
  /** Comma-separated list of enabled Kimi models. */
  kimiModel: string;
  /** Comma-separated list of enabled OpenCode models. */
  opencodeModel: string;
  /** Per-stage model selections. */
  promptEnhancerModel: string;
  plannerModel: string | null;
  planAuditorModel: string | null;
  generatorModel: string;
  reviewerModel: string;
  fixerModel: string;
  finalJudgeModel: string;
  executiveSummaryModel: string;
}

/** Default settings values. */
export const DEFAULT_SETTINGS: AppSettings = {
  claudePath: "claude",
  codexPath: "codex",
  geminiPath: "gemini",
  kimiPath: "kimi",
  opencodePath: "opencode",
  promptEnhancerAgent: "claude",
  plannerAgent: null,
  planAuditorAgent: null,
  generatorAgent: "claude",
  reviewerAgent: "codex",
  fixerAgent: "claude",
  finalJudgeAgent: "codex",
  executiveSummaryAgent: "codex",
  maxIterations: 3,
  requireGit: true,
  claudeModel: "sonnet",
  codexModel: "codex-5.3",
  geminiModel: "gemini-2.5-pro",
  kimiModel: "kimi-k2.5",
  opencodeModel: "opencode/glm-5",
  promptEnhancerModel: "sonnet",
  plannerModel: null,
  planAuditorModel: null,
  generatorModel: "sonnet",
  reviewerModel: "codex-5.3",
  fixerModel: "sonnet",
  finalJudgeModel: "codex-5.3",
  executiveSummaryModel: "codex-5.3",
};

/** Known model options per CLI, keyed by CLI name. */
export const CLI_MODEL_OPTIONS: Record<string, { value: string; label: string }[]> = {
  claude: [
    { value: "sonnet", label: "Sonnet" },
    { value: "opus", label: "Opus" },
    { value: "haiku", label: "Haiku" },
  ],
  codex: [
    { value: "codex-5.3", label: "Codex 5.3" },
  ],
  gemini: [
    { value: "gemini-3.1-pro-preview", label: "Gemini 3.1 Pro" },
    { value: "gemini-3-pro-preview", label: "Gemini 3.0 Pro" },
    { value: "gemini-3-flash-preview", label: "Gemini 3.0 Flash" },
  ],
  kimi: [
    { value: "kimi-k2.5", label: "Kimi K2.5" },
    { value: "kimi-code", label: "Kimi Code" },
  ],
  opencode: [
    { value: "opencode/glm-5", label: "GLM 5" },
    { value: "opencode/glm-4.7", label: "GLM 4.7" },
  ],
};

/** CLI health check result returned from the backend. */
export interface CliHealth {
  claude: CliStatus;
  codex: CliStatus;
  gemini: CliStatus;
  kimi: CliStatus;
  opencode: CliStatus;
}

export interface CliStatus {
  available: boolean;
  path: string;
  error?: string;
}

/** Version and availability information for a single CLI tool. */
export interface CliVersionInfo {
  name: string;
  cliName: string;
  installedVersion?: string;
  latestVersion?: string;
  upToDate: boolean;
  updateCommand: string;
  available: boolean;
  error?: string;
}

/** Aggregate version information for all CLI tools. */
export interface AllCliVersions {
  claude: CliVersionInfo;
  codex: CliVersionInfo;
  gemini: CliVersionInfo;
  kimi: CliVersionInfo;
  opencode: CliVersionInfo;
}

/** Request to start a pipeline run. */
export interface PipelineRequest {
  prompt: string;
  workspacePath: string;
  /** Session ID for this conversation thread. If omitted, a new session is created. */
  sessionId?: string;
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
  kind: "diff" | "plan" | "plan_audit" | "plan_final" | "review" | "judge" | "executive_summary";
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

/** Full session detail with all runs. */
export interface SessionDetail {
  id: string;
  title: string;
  projectPath: string;
  createdAt: string;
  updatedAt: string;
  runs: RunDetail[];
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
  iterations: IterationDetail[];
  questions: QuestionEntry[];
}

/** Full iteration detail with stages. */
export interface IterationDetail {
  number: number;
  verdict?: string;
  judgeReasoning?: string;
  enhancedPrompt?: string;
  plannerPlan?: string;
  auditVerdict?: string;
  auditReasoning?: string;
  auditedPlan?: string;
  reviewOutput?: string;
  reviewUserGuidance?: string;
  fixOutput?: string;
  judgeOutput?: string;
  generateQuestion?: string;
  generateAnswer?: string;
  fixQuestion?: string;
  fixAnswer?: string;
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

/** Stored log entry from the database. */
export interface LogEntry {
  id: number;
  runId: string;
  stage: string;
  line: string;
  stream: string;
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
