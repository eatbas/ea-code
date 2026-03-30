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

export interface PipelineStageStatusEvent {
  conversationId: string;
  stageIndex: number;
  stageName: string;
  status: ConversationStatus;
  agentLabel: string;
  /** When a stage completes, carries the plan file content to replace SSE output. */
  text?: string;
}

export interface PipelineStageOutputDelta {
  conversationId: string;
  stageIndex: number;
  text: string;
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
}
