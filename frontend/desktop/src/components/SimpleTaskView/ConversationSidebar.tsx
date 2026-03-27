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
    <aside className="flex h-full w-72 shrink-0 flex-col border-r border-[#313134] bg-[#141415]">
      <div className="flex items-center justify-between border-b border-[#313134] px-4 py-4">
        <div>
          <p className="text-sm font-semibold text-[#f5f5f5]">Simple Task</p>
          <p className="text-xs text-[#787880]">Explicit agent selection and resume</p>
        </div>
        <button
          type="button"
          onClick={onCreateNew}
          className="rounded-lg border border-[#313134] bg-[#202022] px-3 py-2 text-xs font-medium text-[#f5f5f5] transition-colors hover:bg-[#2a2a2d]"
        >
          New
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-3">
        {loading && conversations.length === 0 && (
          <p className="px-2 py-4 text-xs text-[#787880]">Loading conversations...</p>
        )}
        {!loading && conversations.length === 0 && (
          <p className="px-2 py-4 text-xs text-[#787880]">No conversations yet. Start with a new task.</p>
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
                    ? "border-[#5a5a61] bg-[#242426]"
                    : "border-[#313134] bg-[#19191a] hover:bg-[#202022]"
                }`}
              >
                <p className="truncate text-sm font-medium text-[#f5f5f5]">{conversation.title}</p>
                <p className="mt-1 text-xs text-[#787880]">
                  {providerDisplayName(conversation.agent.provider)} · {conversation.agent.model}
                </p>
                <div className="mt-2 flex items-center justify-between text-[11px] text-[#7e7e86]">
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
