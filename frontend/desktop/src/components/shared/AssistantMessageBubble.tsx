import type { ReactNode } from "react";

interface AssistantMessageBubbleProps {
  /** The assistant message content. */
  content: string;
  /** Optional timestamp for display. */
  timestamp?: string;
}

/** Left-aligned assistant message bubble for the chat timeline. */
export function AssistantMessageBubble({
  content,
  timestamp,
}: AssistantMessageBubbleProps): ReactNode {
  return (
    <div className="flex justify-start">
      <div className="max-w-[80%] rounded-2xl rounded-bl-md bg-[#1a1a2e] px-4 py-3 text-sm text-[#e4e4ed]">
        <p className="whitespace-pre-wrap">{content}</p>
        {timestamp && (
          <span className="mt-1 block text-[10px] text-[#6f7086]">
            {new Date(timestamp).toLocaleTimeString([], {
              hour: "2-digit",
              minute: "2-digit",
            })}
          </span>
        )}
      </div>
    </div>
  );
}
