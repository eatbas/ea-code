import type { ConversationDetail, ConversationSummary } from "../../types";
import { upsertByKey } from "../useEventResource";

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

export function sortConversations(items: ConversationSummary[]): ConversationSummary[] {
  return [...items].sort((left, right) => {
    const pinOrder = Number(Boolean(right.pinnedAt)) - Number(Boolean(left.pinnedAt));
    if (pinOrder !== 0) {
      return pinOrder;
    }

    return right.updatedAt.localeCompare(left.updatedAt);
  });
}

export function promptDraftKey(workspacePath: string, conversationId?: string | null): string {
  return conversationId ? `${workspacePath}::${conversationId}` : `${workspacePath}::__new__`;
}

export function upsertConversationSummary(
  items: ConversationSummary[],
  summary: ConversationSummary,
): ConversationSummary[] {
  return sortConversations(
    upsertByKey(
      items,
      mergeSummary(items.find((item) => item.id === summary.id), summary),
      (item) => item.id,
    ),
  );
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
