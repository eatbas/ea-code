import type { ConversationDetail, ConversationSummary } from "../../types";

export function mergeSummary(
  previous: ConversationSummary | undefined,
  next: ConversationSummary,
): ConversationSummary {
  if (!previous) {
    return next;
  }

  return {
    ...previous,
    ...next,
  };
}

export function promptDraftKey(workspacePath: string, conversationId?: string | null): string {
  return conversationId ? `${workspacePath}::${conversationId}` : `${workspacePath}::__new__`;
}

export function updateActiveConversationSummary(
  previous: ConversationDetail | null,
  summary: ConversationSummary,
): ConversationDetail | null {
  if (!previous || previous.summary.id !== summary.id) {
    return previous;
  }

  return {
    ...previous,
    summary: mergeSummary(previous.summary, summary),
  };
}

export function removeEntry(
  previous: Record<string, string>,
  key: string,
): Record<string, string> {
  const next = { ...previous };
  delete next[key];
  return next;
}
