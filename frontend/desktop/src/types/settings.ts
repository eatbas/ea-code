import type { AgentBackend } from "./agents";

/** Configuration for an extra parallel planner or reviewer slot. */
export interface ExtraSlotConfig {
  agent: AgentBackend | null;
  model: string | null;
}

/** Application settings persisted locally. */
export interface AppSettings {
  /** UI theme preference. */
  theme: "system" | "light" | "dark";
  /** Default agent backend for new sessions. */
  defaultAgent: string | null;
  claudePath: string;
  codexPath: string;
  geminiPath: string;
  kimiPath: string;
  opencodePath: string;
  promptEnhancerAgent: AgentBackend | null;
  skillSelectorAgent: AgentBackend | null;
  plannerAgent: AgentBackend | null;
  planAuditorAgent: AgentBackend | null;
  coderAgent: AgentBackend | null;
  codeReviewerAgent: AgentBackend | null;
  /** Review Merger agent backend. */
  reviewMergerAgent: AgentBackend | null;
  codeFixerAgent: AgentBackend | null;
  finalJudgeAgent: AgentBackend | null;
  executiveSummaryAgent: AgentBackend | null;
  maxIterations: number;
  requireGit: boolean;
  /** Budget mode: skip all planning, send prompt directly to coder. */
  budgetMode: boolean;
  /** Minimum weighted review score to pass (default 7.0). */
  reviewPassScore: number;
  /** Pause pipeline after planning to let the user approve, revise, or skip the plan. */
  requirePlanApproval: boolean;
  /** Seconds to wait before auto-approving the plan (0 = wait indefinitely). */
  planAutoApproveTimeoutSec: number;
  /** Maximum number of plan revision rounds before auto-approving. */
  maxPlanRevisions: number;
  /** Use token-optimised prompt variants (compact handoff, git inspection). */
  tokenOptimisedPrompts: boolean;
  /** Number of retries per agent call on failure (0 = no retries). */
  agentRetryCount: number;
  /** Per-agent timeout in milliseconds (0 = no timeout). */
  agentTimeoutMs: number;
  /** Maximum agentic turns per invocation for CLIs that support it. */
  agentMaxTurns: number;
  /** Completed runs older than this many days are deleted on startup (0 = disabled). */
  retentionDays: number;
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
  skillSelectorModel: string | null;
  plannerModel: string | null;
  planAuditorModel: string | null;
  coderModel: string;
  codeReviewerModel: string;
  /** Model for review merger stage. */
  reviewMergerModel: string | null;
  codeFixerModel: string;
  finalJudgeModel: string;
  executiveSummaryModel: string;
  /** Extra planner slot configurations (planner 2, 3, 4, ...). */
  extraPlanners: ExtraSlotConfig[];
  /** Extra reviewer slot configurations (reviewer 2, 3, 4, ...). */
  extraReviewers: ExtraSlotConfig[];
  /** Maximum total planner slots (1 = primary only, 2+ = primary + extras). */
  maxPlanners: number;
  /** Maximum total reviewer slots (1 = primary only, 2+ = primary + extras). */
  maxReviewers: number;
  /** Port for the hive-api sidecar (0 = default 8719). */
  hiveApiPort: number;
  /** Python interpreter path override for the sidecar (empty = auto-detect). */
  pythonPath: string;
}

/** Default settings values. */
export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  defaultAgent: null,
  claudePath: "claude",
  codexPath: "codex",
  geminiPath: "gemini",
  kimiPath: "kimi",
  opencodePath: "opencode",
  promptEnhancerAgent: null,
  skillSelectorAgent: null,
  plannerAgent: null,
  planAuditorAgent: null,
  coderAgent: null,
  codeReviewerAgent: null,
  reviewMergerAgent: null,
  codeFixerAgent: null,
  finalJudgeAgent: null,
  executiveSummaryAgent: null,
  maxIterations: 3,
  requireGit: true,
  budgetMode: false,
  reviewPassScore: 7.0,
  requirePlanApproval: false,
  planAutoApproveTimeoutSec: 45,
  maxPlanRevisions: 3,
  tokenOptimisedPrompts: false,
  agentRetryCount: 1,
  agentTimeoutMs: 0,
  agentMaxTurns: 25,
  retentionDays: 90,
  claudeModel: "sonnet",
  codexModel: "gpt-5.3-codex",
  geminiModel: "gemini-3-flash-preview",
  kimiModel: "kimi-code/kimi-for-coding",
  opencodeModel: "opencode/glm-5",
  promptEnhancerModel: "sonnet",
  skillSelectorModel: null,
  plannerModel: null,
  planAuditorModel: null,
  coderModel: "sonnet",
  codeReviewerModel: "gpt-5.3-codex",
  reviewMergerModel: null,
  codeFixerModel: "sonnet",
  finalJudgeModel: "gpt-5.3-codex",
  executiveSummaryModel: "gpt-5.3-codex",
  extraPlanners: [],
  extraReviewers: [],
  maxPlanners: 4,
  maxReviewers: 4,
  hiveApiPort: 0,
  pythonPath: "",
};

/** Known model options per CLI, keyed by CLI name. */
export const CLI_MODEL_OPTIONS: Record<string, { value: string; label: string }[]> = {
  claude: [
    { value: "sonnet", label: "Sonnet" },
    { value: "opus", label: "Opus" },
    { value: "haiku", label: "Haiku" },
  ],
  codex: [
    { value: "gpt-5.3-codex", label: "GPT-5.3 Codex" },
    { value: "gpt-5.4", label: "GPT-5.4" },
  ],
  gemini: [
    { value: "gemini-3-flash-preview", label: "Gemini 3 Flash" },
    { value: "gemini-3.1-pro-preview", label: "Gemini 3.1 Pro" },
  ],
  kimi: [
    { value: "kimi-code/kimi-for-coding", label: "Kimi Code" },
  ],
  opencode: [
    { value: "opencode/glm-5", label: "GLM 5" },
    { value: "opencode/glm-4.7", label: "GLM 4.7" },
  ],
};
