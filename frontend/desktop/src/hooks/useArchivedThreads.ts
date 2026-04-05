import { useCallback, useEffect, useState } from "react";
import type { ConversationSummary, ProjectEntry } from "../types";
import { listProjects, listWorkspaceConversations } from "../lib/desktopApi";
import { useToast } from "../components/shared/Toast";

/** A single archived thread enriched with its parent project info. */
export interface ArchivedThread {
  conversation: ConversationSummary;
  projectName: string;
  projectPath: string;
}

interface UseArchivedThreadsReturn {
  threads: ArchivedThread[];
  loading: boolean;
  /** Remove a thread from local state (does NOT call the backend). */
  removeThread: (conversationId: string) => void;
  refresh: () => void;
}

export function useArchivedThreads(): UseArchivedThreadsReturn {
  const [threads, setThreads] = useState<ArchivedThread[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const toast = useToast();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const projects: ProjectEntry[] = await listProjects(true);
      const results = await Promise.allSettled(
        projects.map(async (project) => {
          const conversations = await listWorkspaceConversations(project.path, true);
          return conversations
            .filter((c) => c.archivedAt !== null)
            .map((conversation) => ({
              conversation,
              projectName: project.name,
              projectPath: project.path,
            }));
        }),
      );

      const allThreads: ArchivedThread[] = results
        .filter((r): r is PromiseFulfilledResult<ArchivedThread[]> => r.status === "fulfilled")
        .flatMap((r) => r.value);

      // Sort newest archived first.
      allThreads.sort((a, b) => {
        const aDate = a.conversation.archivedAt ?? "";
        const bDate = b.conversation.archivedAt ?? "";
        return bDate.localeCompare(aDate);
      });

      setThreads(allThreads);
    } catch {
      toast.error("Failed to load archived threads.");
    } finally {
      setLoading(false);
    }
  }, [toast]);

  useEffect(() => {
    void load();
  }, [load]);

  const removeThread = useCallback((conversationId: string) => {
    setThreads((prev) => prev.filter((t) => t.conversation.id !== conversationId));
  }, []);

  return { threads, loading, removeThread, refresh: load };
}
