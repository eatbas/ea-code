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

/** CLI health check result returned from the backend. */
export interface CliHealth {
  claude: CliStatus;
  codex: CliStatus;
  gemini: CliStatus;
  kimi: CliStatus;
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
  opencode: CliVersionInfo;
  gitBash?: CliVersionInfo;
}

// ── hive-api types ─────────────────────────────────────────────────

/** hive-api connection health status. */
export interface ApiHealth {
  connected: boolean;
  url: string;
  status?: string;
  droneCount?: number;
  error?: string;
}

/** Provider availability from hive-api. */
export interface ProviderInfo {
  name: string;
  available: boolean;
  models: string[];
}

/** CLI version info from hive-api. */
export interface ApiCliVersionInfo {
  provider: string;
  installedVersion?: string;
  latestVersion?: string;
  upToDate: boolean;
  available: boolean;
}
