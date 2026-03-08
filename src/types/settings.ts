import type { AgentBackend } from "./agents";

/** Application settings persisted locally. */
export interface AppSettings {
  claudePath: string;
  codexPath: string;
  geminiPath: string;
  kimiPath: string;
  opencodePath: string;
  promptEnhancerAgent: AgentBackend;
  skillSelectorAgent: AgentBackend | null;
  plannerAgent: AgentBackend | null;
  planAuditorAgent: AgentBackend | null;
  generatorAgent: AgentBackend;
  reviewerAgent: AgentBackend;
  fixerAgent: AgentBackend;
  finalJudgeAgent: AgentBackend;
  executiveSummaryAgent: AgentBackend;
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
  generatorModel: string;
  reviewerModel: string;
  fixerModel: string;
  finalJudgeModel: string;
  executiveSummaryModel: string;
  skillSelectionMode: "disable" | "auto";
}

/** Default settings values. */
export const DEFAULT_SETTINGS: AppSettings = {
  claudePath: "claude",
  codexPath: "codex",
  geminiPath: "gemini",
  kimiPath: "kimi",
  opencodePath: "opencode",
  promptEnhancerAgent: "claude",
  skillSelectorAgent: null,
  plannerAgent: null,
  planAuditorAgent: null,
  generatorAgent: "claude",
  reviewerAgent: "codex",
  fixerAgent: "claude",
  finalJudgeAgent: "codex",
  executiveSummaryAgent: "codex",
  maxIterations: 3,
  requireGit: true,
  requirePlanApproval: false,
  planAutoApproveTimeoutSec: 45,
  maxPlanRevisions: 3,
  tokenOptimizedPrompts: false,
  agentRetryCount: 1,
  agentTimeoutMs: 0,
  claudeModel: "sonnet",
  codexModel: "codex-5.3",
  geminiModel: "gemini-2.5-pro",
  kimiModel: "kimi-k2.5",
  opencodeModel: "opencode/glm-5",
  promptEnhancerModel: "sonnet",
  skillSelectorModel: null,
  plannerModel: null,
  planAuditorModel: null,
  generatorModel: "sonnet",
  reviewerModel: "codex-5.3",
  fixerModel: "sonnet",
  finalJudgeModel: "codex-5.3",
  executiveSummaryModel: "codex-5.3",
  skillSelectionMode: "disable",
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
