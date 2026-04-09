import type { ReactNode } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
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
  const { agent: conversationAgent } = activeConversation.summary;

  /** Build the label for an assistant message, using per-message agent if available. */
  function assistantLabelFor(agent?: { provider: string; model: string }, thinkingLevel?: string): string {
    const effectiveAgent = agent ?? conversationAgent;
    const base = formatAssistantLabel(effectiveAgent.provider, effectiveAgent.model);
    return thinkingLevel ? `${base} · ${thinkingLevel}` : base;
  }

  // Fallback label for the live draft (uses current conversation agent).
  const draftLabel = assistantLabelFor();

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-4">
      {activeConversation.messages.map((message) =>
        message.role === "user" ? (
          <div
            key={message.id}
            className="ml-auto flex max-w-3xl flex-col items-end"
          >
            <div className="rounded-2xl border border-edge-strong bg-elevated px-4 py-3 text-sm leading-6 text-fg">
              <p className="mb-1 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
                {message.role}
              </p>
              <p className="whitespace-pre-wrap">{message.content}</p>
            </div>
            <p className="mt-1 px-1 text-[10px] text-fg-muted">
              {new Date(message.createdAt).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
            </p>
          </div>
        ) : (
          <div key={message.id} className="mr-auto flex w-full flex-col">
            <p className="mb-2 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
              {`Assistant - ${assistantLabelFor(message.agent, message.thinkingLevel)}`}
            </p>
            <div className="assistant-prose text-sm leading-7 text-fg">
              <Markdown remarkPlugins={[remarkGfm]}>{message.content}</Markdown>
            </div>
            <p className="mt-2 text-[10px] text-fg-muted">
              {new Date(message.createdAt).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}
            </p>
          </div>
        ),
      )}

      {activeDraft && (
        <div className="mr-auto flex w-full flex-col">
          <p className="mb-2 text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
            {`Assistant - ${draftLabel}`}
          </p>
          <div className="assistant-prose text-sm leading-7 text-fg">
            <Markdown remarkPlugins={[remarkGfm]}>{activeDraft}</Markdown>
          </div>
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
