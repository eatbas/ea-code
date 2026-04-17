export interface AgentSelection {
  provider: string;
  model: string;
}

export type ConversationStatus = "idle" | "running" | "completed" | "failed" | "stopped" | "awaiting_review";

export type ConversationMessageRole = "user" | "assistant";

export interface ConversationMessage {
  id: string;
  role: ConversationMessageRole;
  content: string;
  createdAt: string;
  /** The agent that produced this message (assistant messages only). */
  agent?: AgentSelection;
  /** Thinking level active when this message was produced (assistant only). */
  thinkingLevel?: string;
}

export interface ConversationSummary {
  id: string;
  title: string;
  workspacePath: string;
  agent: AgentSelection;
  status: ConversationStatus;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  lastProviderSessionRef: string | null;
  activeScoreId: string | null;
  error: string | null;
  archivedAt: string | null;
  pinnedAt: string | null;
}

export interface ConversationDetail {
  summary: ConversationSummary;
  messages: ConversationMessage[];
}

export interface ConversationOutputDelta {
  conversationId: string;
  text: string;
}

export interface ConversationStatusEvent {
  conversation: ConversationSummary;
  message?: ConversationMessage;
}

export interface ConversationDeletedEvent {
  workspacePath: string;
  conversationId: string;
}

export interface PipelineStageStatusEvent {
  conversationId: string;
  stageIndex: number;
  stageName: string;
  status: ConversationStatus;
  agentLabel: string;
  /** When a stage completes, carries the plan file content to replace SSE output. */
  text?: string;
  /** Persisted start time — present when re-emitting saved stages. */
  startedAt?: string;
  /** Persisted finish time — present when re-emitting saved stages. */
  finishedAt?: string;
}

export interface PipelineStageOutputDelta {
  conversationId: string;
  stageIndex: number;
  text: string;
}

export interface PipelineDebugLogEvent {
  conversationId: string;
  createdAt: string;
  line: string;
}

export interface PipelineStageRecord {
  stageIndex: number;
  stageName: string;
  agentLabel: string;
  status: ConversationStatus;
  text: string;
  startedAt: string | null;
  finishedAt: string | null;
  scoreId?: string | null;
  providerSessionRef?: string | null;
}

export interface PipelineState {
  userPrompt: string;
  pipelineMode: string;
  stages: PipelineStageRecord[];
  enhancedPrompt?: string;
}

export interface ImageSaveResult {
  fileName: string;
  filePath: string;
}

export interface ImageEntry {
  fileName: string;
  filePath: string;
}
