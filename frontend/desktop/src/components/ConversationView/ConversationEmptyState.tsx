import type { ReactNode } from "react";

export function ConversationEmptyState(): ReactNode {
  return (
    <div className="mx-auto flex h-full max-w-3xl items-center justify-center">
      <div className="rounded-3xl border border-edge bg-panel px-8 py-10 text-center shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        <img src="/logo_2.png" alt="Maestro logo" className="mx-auto mb-4 h-64 w-64 object-contain" />
        <p className="text-xl font-semibold text-fg">Start a new conversation</p>
        <p className="mt-2 text-sm text-fg-muted">
          Pick an agent in the composer and send the first prompt to start a conversation.
        </p>
      </div>
    </div>
  );
}
