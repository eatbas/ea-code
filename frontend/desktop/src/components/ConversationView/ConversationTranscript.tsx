import type { ReactNode } from "react";
import type { ConversationDetail } from "../../types";
import { formatAssistantLabel } from "../shared/constants";

interface ConversationTranscriptProps {
  activeConversation: ConversationDetail;
  activeDraft: string;
}

export function ConversationTranscript({
  activeConversation,
  activeDraft,
}: ConversationTranscriptProps): ReactNode {
  const assistantLabel = formatAssistantLabel(
    activeConversation.summary.agent.provider,
    activeConversation.summary.agent.model,
  );

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-4">
      {activeConversation.messages.map((message) => (
        <div
          key={message.id}
          className={`flex max-w-3xl flex-col ${message.role === "user" ? "ml-auto items-end" : "mr-auto items-end"}`}
        >
          <div
            className={`rounded-2xl px-4 py-3 text-sm leading-6 ${
              message.role === "user"
                ? "border border-edge-strong bg-elevated text-fg"
                : "border border-edge bg-panel text-fg"
            }`}
          >
            <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
              {message.role === "assistant" ? `Assistant - ${assistantLabel}` : message.role}
            </p>
            <p className="whitespace-pre-wrap">{message.content}</p>
          </div>
          <p className="mt-1 px-1 text-[10px] text-fg-muted">
            {new Date(message.createdAt).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
          </p>
        </div>
      ))}

      {activeDraft && (
        <div className="mr-auto max-w-3xl rounded-2xl border border-dashed border-edge-strong bg-input-bg px-4 py-3 text-sm leading-6 text-fg">
          <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
            {`Assistant - ${assistantLabel}`}
          </p>
          <p className="whitespace-pre-wrap">{activeDraft}</p>
        </div>
      )}

      {activeConversation.summary.error && activeConversation.summary.status === "failed" && (
        <div className="mr-auto max-w-3xl rounded-2xl border border-error-border bg-error-bg px-4 py-3 text-sm text-error-text">
          {activeConversation.summary.error}
        </div>
      )}
    </div>
  );
}
