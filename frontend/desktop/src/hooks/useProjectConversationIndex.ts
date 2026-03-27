import { useEffect, useState } from "react";
import type { ConversationStatusEvent, ConversationSummary, ProjectEntry } from "../types";
import { listWorkspaceConversations } from "../lib/desktopApi";
import { CONVERSATION_EVENTS } from "../constants/events";
import { useTauriEventListeners } from "./useTauriEventListeners";
import { upsertByKey } from "./useEventResource";

type ConversationIndex = Record<string, ConversationSummary[]>;

interface UseProjectConversationIndexReturn {
  index: ConversationIndex;
  removeConversation: (workspacePath: string, conversationId: string) => void;
}

function sortConversations(items: ConversationSummary[]): ConversationSummary[] {
  return [...items].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function useProjectConversationIndex(projects: ProjectEntry[]): UseProjectConversationIndexReturn {
  const [index, setIndex] = useState<ConversationIndex>({});

  useTauriEventListeners({
    listeners: [
      {
        event: CONVERSATION_EVENTS.STATUS,
        handler: (payload: ConversationStatusEvent) => {
          setIndex((previous) => ({
            ...previous,
            [payload.conversation.workspacePath]: sortConversations(
              upsertByKey(
                previous[payload.conversation.workspacePath] ?? [],
                payload.conversation,
                (item) => item.id,
              ),
            ),
          }));
        },
      },
    ],
  });

  useEffect(() => {
    let cancelled = false;

    async function loadConversations(): Promise<void> {
      const entries = await Promise.all(projects.map(async (project) => {
        try {
          const conversations = await listWorkspaceConversations(project.path);
          return [project.path, sortConversations(conversations)] as const;
        } catch {
          return [project.path, []] as const;
        }
      }));

      if (!cancelled) {
        setIndex(Object.fromEntries(entries));
      }
    }

    void loadConversations();

    return () => {
      cancelled = true;
    };
  }, [projects]);

  return {
    index,
    removeConversation: (workspacePath: string, conversationId: string) => {
      setIndex((previous) => ({
        ...previous,
        [workspacePath]: (previous[workspacePath] ?? []).filter((conversation) => conversation.id !== conversationId),
      }));
    },
  };
}
