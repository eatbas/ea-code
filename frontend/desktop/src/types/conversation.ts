export interface AgentSelection {
  provider: string;
  model: string;
}

export type ConversationStatus = "idle" | "running" | "completed" | "failed" | "stopped";

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
  activeJobId: string | null;
  error: string | null;
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
