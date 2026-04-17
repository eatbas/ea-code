import { useCallback, useEffect, useRef, useState } from "react";
import type {
  ConversationDeletedEvent,
  ConversationStatusEvent,
  ConversationSummary,
  ProjectEntry,
} from "../types";
import { listWorkspaceConversations } from "../lib/desktopApi";
import { CONVERSATION_EVENTS } from "../constants/events";
import { useTauriEventListeners } from "./useTauriEventListeners";
import { upsertByKey } from "./useEventResource";

type ConversationIndex = Record<string, ConversationSummary[]>;

interface UseProjectConversationIndexReturn {
  index: ConversationIndex;
  loadedProjectPaths: Set<string>;
  loadingProjectPaths: Set<string>;
  ensureLoaded: (workspacePath: string) => Promise<void>;
}

function sortConversations(items: ConversationSummary[]): ConversationSummary[] {
  return [...items].sort((left, right) => {
    const archiveOrder = Number(Boolean(left.archivedAt)) - Number(Boolean(right.archivedAt));
    if (archiveOrder !== 0) {
      return archiveOrder;
    }
    const pinOrder = Number(Boolean(right.pinnedAt)) - Number(Boolean(left.pinnedAt));
    if (pinOrder !== 0) {
      return pinOrder;
    }
    return right.updatedAt.localeCompare(left.updatedAt);
  });
}

export function useProjectConversationIndex(projects: ProjectEntry[]): UseProjectConversationIndexReturn {
  const [index, setIndex] = useState<ConversationIndex>({});
  const [loadedProjectPaths, setLoadedProjectPaths] = useState<Set<string>>(new Set());
  const [loadingProjectPaths, setLoadingProjectPaths] = useState<Set<string>>(new Set());
  const loadedProjectPathsRef = useRef<Set<string>>(new Set());
  const pendingLoadsRef = useRef<Record<string, Promise<void>>>({});

  const markProjectLoaded = useCallback((workspacePath: string): void => {
    loadedProjectPathsRef.current = new Set(loadedProjectPathsRef.current).add(workspacePath);
    setLoadedProjectPaths((current) => (
      current.has(workspacePath) ? current : new Set(current).add(workspacePath)
    ));
  }, []);

  useTauriEventListeners({
    listeners: [
      {
        event: CONVERSATION_EVENTS.STATUS,
        handler: (payload: ConversationStatusEvent) => {
          markProjectLoaded(payload.conversation.workspacePath);
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
      {
        event: CONVERSATION_EVENTS.DELETED,
        handler: (payload: ConversationDeletedEvent) => {
          setIndex((previous) => {
            const existing = previous[payload.workspacePath];
            if (!existing) {
              return previous;
            }
            return {
              ...previous,
              [payload.workspacePath]: existing.filter(
                (conversation) => conversation.id !== payload.conversationId,
              ),
            };
          });
        },
      },
    ],
  });

  useEffect(() => {
    const validProjectPaths = new Set(projects.map((project) => project.path));

    setIndex((current) => {
      const entries = Object.entries(current).filter(([projectPath]) => validProjectPaths.has(projectPath));
      return entries.length === Object.keys(current).length ? current : Object.fromEntries(entries);
    });
    setLoadedProjectPaths((current) => {
      const next = new Set([...current].filter((projectPath) => validProjectPaths.has(projectPath)));
      loadedProjectPathsRef.current = next;
      return next.size === current.size ? current : next;
    });
    setLoadingProjectPaths((current) => {
      const next = new Set([...current].filter((projectPath) => validProjectPaths.has(projectPath)));
      return next.size === current.size ? current : next;
    });
  }, [projects]);

  const ensureLoaded = useCallback(async (workspacePath: string): Promise<void> => {
    if (loadedProjectPathsRef.current.has(workspacePath)) {
      return;
    }

    const pendingLoad = pendingLoadsRef.current[workspacePath];
    if (pendingLoad) {
      await pendingLoad;
      return;
    }

    const loadPromise = (async () => {
      setLoadingProjectPaths((current) => (
        current.has(workspacePath) ? current : new Set(current).add(workspacePath)
      ));

      try {
        const conversations = await listWorkspaceConversations(workspacePath, true);
        setIndex((previous) => ({
          ...previous,
          [workspacePath]: sortConversations(conversations),
        }));
      } catch {
        setIndex((previous) => (
          workspacePath in previous ? previous : { ...previous, [workspacePath]: [] }
        ));
      } finally {
        markProjectLoaded(workspacePath);
        setLoadingProjectPaths((current) => {
          if (!current.has(workspacePath)) {
            return current;
          }

          const next = new Set(current);
          next.delete(workspacePath);
          return next;
        });
        delete pendingLoadsRef.current[workspacePath];
      }
    })();

    pendingLoadsRef.current[workspacePath] = loadPromise;
    await loadPromise;
  }, [markProjectLoaded]);

  return {
    index,
    loadedProjectPaths,
    loadingProjectPaths,
    ensureLoaded,
  };
}
