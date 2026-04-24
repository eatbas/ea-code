/** Supported CLI agent backends (dynamic — any provider name from Symphony). */
export type AgentBackend = string;

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

/** Startup prerequisite check result. */
export interface PrerequisiteStatus {
  pythonAvailable: boolean;
  pythonVersion?: string;
  /** Windows-only — always `true` on macOS/Linux. */
  gitBashAvailable: boolean;
  symphonySourceFound: boolean;
}

/** Symphony connection health status. */
export interface ApiHealth {
  connected: boolean;
  url: string;
  status?: string;
  configPath?: string;
  shellPath?: string;
  bashVersion?: string;
  musiciansBooted?: boolean;
  musicianCount?: number;
  details?: string[];
  error?: string;
}

/** Provider option choice exposed by the API. */
export interface ProviderOptionChoice {
  value: string;
  label: string;
  description?: string;
}

/** Provider option definition exposed by the API for a specific model. */
export interface ProviderOptionDefinition {
  key: string;
  label: string;
  type: "select";
  default?: string;
  choices: ProviderOptionChoice[];
}

/** Detailed model information from Symphony `/v1/models`. */
export interface ModelDetail {
  provider: string;
  model: string;
  ready: boolean;
  busy: boolean;
  supportsResume: boolean;
  providerOptionsSchema: ProviderOptionDefinition[];
}

/** Provider availability from Symphony. */
export interface ProviderInfo {
  name: string;
  executable?: string;
  enabled: boolean;
  available: boolean;
  models: string[];
  supportsResume: boolean;
  supportsModelOverride: boolean;
  sessionReferenceFormat: string;
}

/** CLI version info from Symphony. */
export interface ApiCliVersionInfo {
  provider: string;
  executable?: string;
  installedVersion?: string;
  latestVersion?: string;
  upToDate: boolean;
  available: boolean;
  needsUpdate?: boolean;
  lastChecked?: string;
  nextCheckAt?: string;
  autoUpdate?: boolean;
  lastUpdated?: string;
  updateSkippedReason?: string;
}

/** A single sidecar stdout/stderr log entry. */
export interface SidecarLogEntry {
  stream: string;
  line: string;
  timestamp: string;
}
