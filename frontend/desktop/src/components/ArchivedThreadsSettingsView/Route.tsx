import type { ReactNode } from "react";
import { useCallback } from "react";
import { useArchivedThreads } from "../../hooks/useArchivedThreads";
import { ArchivedThreadsSettingsView } from ".";

interface ArchivedThreadsSettingsRouteProps {
  onUnarchiveConversation: (projectPath: string, conversationId: string) => void;
}

export function ArchivedThreadsSettingsRoute({ onUnarchiveConversation }: ArchivedThreadsSettingsRouteProps): ReactNode {
  const { threads, loading, removeThread } = useArchivedThreads();

  const handleUnarchive = useCallback(
    async (projectPath: string, conversationId: string) => {
      onUnarchiveConversation(projectPath, conversationId);
      removeThread(conversationId);
    },
    [onUnarchiveConversation, removeThread],
  );

  return (
    <ArchivedThreadsSettingsView
      threads={threads}
      loading={loading}
      onUnarchive={handleUnarchive}
    />
  );
}
