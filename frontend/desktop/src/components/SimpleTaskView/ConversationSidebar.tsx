import type { ReactNode } from "react";
import type { ConversationSummary } from "../../types";
import { providerDisplayName } from "../shared/constants";

interface ConversationSidebarProps {
  conversations: ConversationSummary[];
  activeConversationId: string | null;
  loading: boolean;
  onSelect: (conversationId: string) => Promise<void>;
  onCreateNew: () => void;
}

function statusLabel(status: ConversationSummary["status"]): string {
  switch (status) {
    case "running":
      return "Running";
    case "completed":
      return "Completed";
    case "failed":
      return "Failed";
    case "stopped":
      return "Stopped";
    default:
      return "Idle";
  }
}

export function ConversationSidebar({
  conversations,
  activeConversationId,
  loading,
  onSelect,
  onCreateNew,
}: ConversationSidebarProps): ReactNode {
  return (
    <aside className="flex h-full w-72 shrink-0 flex-col border-r border-[#2e2e48] bg-[#161620]">
      <div className="flex items-center justify-between border-b border-[#2e2e48] px-4 py-4">
        <div>
          <p className="text-sm font-semibold text-[#e4e4ed]">Simple Task</p>
          <p className="text-xs text-[#7f819a]">Explicit agent selection and resume</p>
        </div>
        <button
          type="button"
          onClick={onCreateNew}
          className="rounded-lg border border-[#2e2e48] bg-[#1f1f2c] px-3 py-2 text-xs font-medium text-[#e4e4ed] transition-colors hover:bg-[#28283a]"
        >
          New
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-3">
        {loading && conversations.length === 0 && (
          <p className="px-2 py-4 text-xs text-[#7f819a]">Loading conversations...</p>
        )}
        {!loading && conversations.length === 0 && (
          <p className="px-2 py-4 text-xs text-[#7f819a]">No conversations yet. Start with a new task.</p>
        )}
        <div className="space-y-2">
          {conversations.map((conversation) => {
            const isActive = conversation.id === activeConversationId;
            return (
              <button
                key={conversation.id}
                type="button"
                onClick={() => {
                  void onSelect(conversation.id);
                }}
                className={`w-full rounded-xl border px-3 py-3 text-left transition-colors ${
                  isActive
                    ? "border-[#4f46e5] bg-[#222136]"
                    : "border-[#2e2e48] bg-[#191927] hover:bg-[#202033]"
                }`}
              >
                <p className="truncate text-sm font-medium text-[#e4e4ed]">{conversation.title}</p>
                <p className="mt-1 text-xs text-[#7f819a]">
                  {providerDisplayName(conversation.agent.provider)} · {conversation.agent.model}
                </p>
                <div className="mt-2 flex items-center justify-between text-[11px] text-[#8f91a7]">
                  <span>{statusLabel(conversation.status)}</span>
                  <span>{conversation.messageCount} messages</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </aside>
  );
}
