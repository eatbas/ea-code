/** A single agent slot within a pipeline stage. */
export interface PipelineAgent {
  provider: string;
  model: string;
}

/** Orchestrator agent that enhances prompts and routes to the right pipeline. */
export interface OrchestratorSettings {
  /** Fast agent used for prompt enhancement and pipeline routing. */
  agent: PipelineAgent;
  /** Maximum review-fix iterations before stopping (default 3). */
  maxIterations: number;
}

/** Configuration for the multi-stage code pipeline. */
export interface CodePipelineSettings {
  /** Agents that plan in parallel, each producing Plan-N.md. */
  planners: PipelineAgent[];
  /** Single agent that writes code (also used by the fixer via resume). */
  coder: PipelineAgent;
  /** Agents that review the code. */
  reviewers: PipelineAgent[];
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
  /** Per-provider enabled models (e.g. { copilot: "claude-sonnet-4.6,gpt-5.4" }). */
  providerModels: Record<string, string>;
  /** Port for the Symphony sidecar (0 = default 8719). */
  symphonyPort: number;
  /** Python interpreter path override (empty = auto-detect). */
  pythonPath: string;
  /** Orchestrator configuration (null = not configured). */
  orchestrator: OrchestratorSettings | null;
  /** Code pipeline configuration (null = not configured). */
  codePipeline: CodePipelineSettings | null;
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
  claudeModel: "sonnet",
  codexModel: "gpt-5.3-codex",
  geminiModel: "gemini-3-flash-preview",
  kimiModel: "kimi-code/kimi-for-coding",
  opencodeModel: "opencode/glm-5",
  providerModels: {},
  symphonyPort: 0,
  pythonPath: "",
  orchestrator: null,
  codePipeline: null,
};
