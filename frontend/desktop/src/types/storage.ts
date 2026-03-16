/** Types for the new file-based storage system.
 *
 * These types mirror the Rust serde structs exactly.
 * All timestamp fields are RFC 3339 strings.
 */

import type { AgentBackend } from "./agents";
import type { PipelineStage, StageStatus, JudgeVerdict } from "./pipeline";
import type { RunStatus } from "./events";

export type { PipelineStage, StageStatus, JudgeVerdict, RunStatus };

// ============================================================================
// Top-level config files
// ============================================================================

/** Contents of settings.json — application settings. */
export interface SettingsFile {
  /** Schema version for migration handling. */
  schemaVersion: number;
  /** CLI paths and agent assignments. */
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
  /** Pipeline configuration. */
  maxIterations: number;
  requireGit: boolean;
  requirePlanApproval: boolean;
  planAutoApproveTimeoutSec: number;
  maxPlanRevisions: number;
  tokenOptimisedPrompts: boolean;
  agentRetryCount: number;
  agentTimeoutMs: number;
  agentMaxTurns: number;
  retentionDays: number;
  /** Model selections. */
  claudeModel: string;
  codexModel: string;
  geminiModel: string;
  kimiModel: string;
  opencodeModel: string;
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

/** Single entry in projects.json — recent project list. */
export interface ProjectEntry {
  /** Unique identifier for the project entry. */
  id: string;
  /** Absolute path to the project directory. */
  path: string;
  /** Display name (directory name). */
  name: string;
  /** Whether the project is a git repository. */
  isGitRepo: boolean;
  /** Current git branch if applicable. */
  branch?: string;
  /** Last opened timestamp (RFC 3339). */
  lastOpened?: string;
  /** When the project entry was first created (RFC 3339). */
  createdAt: string;
}

/** Contents of skills/<id>.json — skill definition. */
export interface SkillFile {
  /** Unique identifier for the skill. */
  id: string;
  /** Display name. */
  name: string;
  /** Short description of what the skill does. */
  description: string;
  /** Full instructions/prompt for the skill. */
  prompt: string;
  /** Comma-separated tags for categorisation. */
  tags: string[];
  /** Whether the skill is currently active. */
  isActive: boolean;
  /** Creation timestamp (RFC 3339). */
  createdAt: string;
  /** Last update timestamp (RFC 3339). */
  updatedAt: string;
}

/** MCP server configuration entry within mcp.json. */
export interface McpServerConfig {
  command: string;
  args?: string[];
  env?: Record<string, string>;
}

/** Contents of mcp.json — MCP servers and CLI bindings. */
export interface McpConfigFile {
  /** Schema version for migration handling. */
  schemaVersion: number;
  /** Map of server ID to MCP server configuration. */
  servers: Record<string, McpServerConfig>;
  /** CLI to server bindings: CLI name -> list of MCP server IDs. */
  cliBindings: Record<string, string[]>;
}

// ============================================================================
// Session and run storage
// ============================================================================

/** Contents of sessions/<id>/session.json — rich session metadata.
 *
 * Contains enough information for the sidebar and session list
 * without scanning run logs.
 */
export interface SessionMeta {
  /** Unique session identifier (UUID). */
  id: string;
  /** Display title for the session. */
  title: string;
  /** Project identifier (UUID) that owns this session. */
  projectId: string;
  /** Absolute path to the project this session belongs to. */
  projectPath: string;
  /** Number of runs in this session. */
  runCount: number;
  /** The most recent prompt submitted in this session. */
  lastPrompt?: string;
  /** Status of the most recent run. */
  lastStatus?: string;
  /** Verdict of the most recent run. */
  lastVerdict?: string;
  /** Creation timestamp (RFC 3339). */
  createdAt: string;
  /** Last update timestamp (RFC 3339). */
  updatedAt: string;
}

/** Git baseline captured at run start for change tracking. */
export interface GitBaseline {
  /** HEAD commit SHA at run start. */
  commitSha: string;
  /** Whether the working tree had unstaged changes at run start. */
  hadUnstagedChanges: boolean;
}

/** Contents of sessions/<id>/runs/<rid>/summary.json — run summary.
 *
 * Written/updated at run end. Also serves as the live run snapshot
 * during execution.
 */
export interface RunSummaryFile {
  /** Schema version for migration handling. */
  schemaVersion: number;
  /** Unique run identifier (UUID). */
  id: string;
  /** Parent session identifier (UUID). */
  sessionId: string;
  /** The original user prompt. */
  prompt: string;
  /** The enhanced/expanded prompt (if enhancement was run). */
  enhancedPrompt?: string;
  /** Current run status. */
  status: RunStatus;
  /** Final judge verdict (null if not complete). */
  finalVerdict: JudgeVerdict | null;
  /** Currently executing stage (null if not running). */
  currentStage: PipelineStage | null;
  /** Current iteration number (1-based). */
  currentIteration: number | null;
  /** Total iterations executed (including incomplete final). */
  totalIterations: number;
  /** Maximum iterations allowed for this run. */
  maxIterations: number;
  /** Executive summary of what was done. */
  executiveSummary?: string;
  /** List of files changed during the run. */
  filesChanged: string[];
  /** Error message if status is failed or crashed. */
  error?: string;
  /** Git baseline at run start for change detection. */
  gitBaseline?: GitBaseline;
  /** Path to the workspace/project directory where git commands should be executed. */
  workspacePath?: string;
  /** Next sequence number for events (avoids reading entire events.jsonl file). */
  nextSequence?: number;
  /** Run start timestamp (RFC 3339). */
  startedAt: string;
  /** Run completion timestamp (RFC 3339) — null if still running. */
  completedAt: string | null;
}

// ============================================================================
// Compact review findings (passed to Judge instead of raw output)
// ============================================================================

/** Compact structured review findings for the Judge.
 *
 * Instead of passing the full raw reviewer output (2000+ tokens),
 * extract only the essential findings for the Judge to evaluate.
 */
export interface ReviewFindings {
  /** List of unresolved blockers that must be fixed. */
  blockers: string[];
  /** List of warnings that should be noted but aren't blocking. */
  warnings: string[];
  /** List of minor suggestions (nits). */
  nits: string[];
  /** Whether tests were run during the review. */
  testsRun: boolean;
  /** Test results summary lines (e.g., "5 passed, 1 failed"). */
  testResults: string[];
  /** The reviewer's overall verdict (PASS or FAIL). */
  verdict: "PASS" | "FAIL";
}

// ============================================================================
// Chat messages (session-level conversation log)
// ============================================================================

/** Role in a chat message. */
export type ChatRole = "user" | "assistant";

/** A single chat message from messages.jsonl at the session level. */
export interface ChatMessage {
  /** Who sent the message. */
  role: ChatRole;
  /** Message content text. */
  content: string;
  /** Timestamp in RFC 3339 format. */
  timestamp: string;
  /** Associated pipeline run ID (if this message triggered/resulted from a run). */
  runId?: string;
}
