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
  /** Port for the hive-api sidecar (0 = default 8719). */
  hiveApiPort: number;
  /** Python interpreter path override (empty = auto-detect). */
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
  claudeModel: "sonnet",
  codexModel: "gpt-5.3-codex",
  geminiModel: "gemini-3-flash-preview",
  kimiModel: "kimi-code/kimi-for-coding",
  opencodeModel: "opencode/glm-5",
  providerModels: {},
  hiveApiPort: 0,
  pythonPath: "",
};
