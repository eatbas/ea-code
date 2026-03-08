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
export type AgentBackend = "claude" | "codex" | "gemini" | "kimi" | "copilot" | "opencode";

/** CLI health check result returned from the backend. */
export interface CliHealth {
  claude: CliStatus;
  codex: CliStatus;
  gemini: CliStatus;
  kimi: CliStatus;
  copilot: CliStatus;
  opencode: CliStatus;
}

/** Availability status for a single CLI tool. */
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
  copilot: CliVersionInfo;
  opencode: CliVersionInfo;
}
