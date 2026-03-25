import type { AgentBackend } from "./agents";
import type { SessionMeta, RunSummaryFile, ProjectEntry, ChatMessage } from "./storage";
import type { RunEvent } from "./events";

/** Options passed from IdleView to App when submitting a prompt. */
export interface RunOptions {
  prompt: string;
  directTask: boolean;
  directTaskAgent?: AgentBackend;
  directTaskModel?: string;
  noPlan: boolean;
}

/** Request to start a pipeline run. */
export interface PipelineRequest {
  prompt: string;
  workspacePath: string;
  /** Session ID for this conversation thread. If omitted, a new session is created. */
  sessionId?: string;
  /** When true, bypass the pipeline and send the prompt directly to a single agent. */
  directTask?: boolean;
  /** Agent backend to use for direct task mode. */
  directTaskAgent?: AgentBackend;
  /** Model to use for direct task mode. */
  directTaskModel?: string;
  /** When true, skip the plan and plan_audit stages in the pipeline. */
  noPlan?: boolean;
}

/** Workspace validation result. */
export interface WorkspaceInfo {
  path: string;
  isGitRepo: boolean;
  branch?: string;
}

// ---- History / persistence types ----

/** Project bookmark - re-export from storage for backwards compatibility. */
export type ProjectSummary = ProjectEntry;

/** Session summary - re-export from storage (SessionMeta replaces SessionSummary). */
export type SessionSummary = SessionMeta;

/** Run summary - re-export from storage (RunSummaryFile is the storage shape). */
export type RunSummary = RunSummaryFile;

/** Full session detail with paginated runs.
 *
 * Note: runs only contain summaries (no events) for efficient loading.
 * Use get_run_events to lazy-load events for individual runs.
 */
export interface SessionDetail {
  id: string;
  title: string;
  projectPath: string;
  createdAt: string;
  updatedAt: string;
  runs: RunSummary[];
  /** Total number of runs in this session (for pagination). */
  totalRuns: number;
  /** Chat messages for this session (from messages.jsonl). */
  messages: ChatMessage[];
}

/** Full run detail with events timeline.
 *
 * Mirrors the Rust RunDetail struct: { summary: RunSummary, events: Vec<RunEvent> }
 */
export interface RunDetail {
  /** Run summary from summary.json - contains all run metadata. */
  summary: RunSummaryFile;
  /** Event timeline from events.jsonl - contains stage timing and status. */
  events: RunEvent[];
}
// Re-export storage types for convenience
export type { SessionMeta, RunSummaryFile, RunEvent, ProjectEntry, ChatMessage };
