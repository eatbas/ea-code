import type { AgentBackend } from "./agents";

/** Application settings persisted locally. */
export interface AppSettings {
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
  codeFixerAgent: AgentBackend | null;
  finalJudgeAgent: AgentBackend | null;
  executiveSummaryAgent: AgentBackend | null;
  maxIterations: number;
  requireGit: boolean;
  /** Pause pipeline after planning to let the user approve, revise, or skip the plan. */
  requirePlanApproval: boolean;
  /** Seconds to wait before auto-approving the plan (0 = wait indefinitely). */
  planAutoApproveTimeoutSec: number;
  /** Maximum number of plan revision rounds before auto-approving. */
  maxPlanRevisions: number;
  /** Use token-optimised prompt variants (compact handoff, git inspection). */
  tokenOptimizedPrompts: boolean;
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
  codeFixerModel: string;
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
  promptEnhancerAgent: null,
  skillSelectorAgent: null,
  plannerAgent: null,
  planAuditorAgent: null,
  coderAgent: null,
  codeReviewerAgent: null,
  codeFixerAgent: null,
  finalJudgeAgent: null,
  executiveSummaryAgent: null,
  maxIterations: 3,
  requireGit: true,
  requirePlanApproval: false,
  planAutoApproveTimeoutSec: 45,
  maxPlanRevisions: 3,
  tokenOptimizedPrompts: false,
  agentRetryCount: 1,
  agentTimeoutMs: 0,
  agentMaxTurns: 25,
  retentionDays: 90,
  claudeModel: "sonnet",
  codexModel: "gpt-5.3-codex",
  geminiModel: "gemini-2.5-pro",
  kimiModel: "kimi-code/kimi-for-coding",
  opencodeModel: "opencode/glm-5",
  promptEnhancerModel: "sonnet",
  skillSelectorModel: null,
  plannerModel: null,
  planAuditorModel: null,
  coderModel: "sonnet",
  codeReviewerModel: "gpt-5.3-codex",
  codeFixerModel: "sonnet",
  finalJudgeModel: "gpt-5.3-codex",
  executiveSummaryModel: "gpt-5.3-codex",
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
    { value: "gemini-3.0-flash", label: "Gemini 3.0 Flash" },
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
