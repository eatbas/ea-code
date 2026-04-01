import type { ReactNode } from "react";
import type { ConversationDetail, WorkspaceInfo } from "../../types";

interface ConversationHeaderProps {
  workspace: WorkspaceInfo;
  activeConversation: ConversationDetail | null;
}

export function ConversationHeader({
  workspace,
  activeConversation,
}: ConversationHeaderProps): ReactNode {
  return (
    <>
      <div className="border-b border-edge bg-[linear-gradient(180deg,var(--color-input-bg)_0%,var(--color-surface)_100%)] px-5 py-4">
        <p className="text-lg font-semibold text-fg">
          {activeConversation?.summary.title ?? "New conversation"}
        </p>
        {!activeConversation && (
          <p className="mt-1 text-sm text-fg-muted">{workspace.path}</p>
        )}
      </div>

      {workspace.isGitRepo && workspace.maestroIgnored === false && (
        <div className="border-b border-warning-border bg-new-btn-bg-hover px-5 py-3 text-sm text-warning-text">
          `.maestro/` is not currently ignored in this repository. Maestro will attempt to add it to `.gitignore`.
        </div>
      )}
    </>
  );
}
