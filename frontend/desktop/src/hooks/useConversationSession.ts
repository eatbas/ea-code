import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  AgentSelection,
  ConversationDetail,
  ConversationOutputDelta,
  ConversationStatusEvent,
  ConversationSummary,
  WorkspaceInfo,
} from "../types";
import { CONVERSATION_EVENTS } from "../constants/events";
import {
  createConversation,
  deleteConversation,
  getConversation,
  listWorkspaceConversations,
  sendConversationTurn,
  stopConversation,
} from "../lib/desktopApi";
import { useTauriEventListeners } from "./useTauriEventListeners";
import { upsertByKey } from "./useEventResource";
import { useToast } from "../components/shared/Toast";

interface UseConversationSessionReturn {
  conversations: ConversationSummary[];
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  loading: boolean;
  sending: boolean;
  stopping: boolean;
  openConversation: (conversationId: string) => Promise<void>;
  startNewConversation: () => void;
  sendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  stopActiveConversation: () => Promise<void>;
  deleteConversationById: (conversationId: string) => Promise<void>;
}

export interface ConversationSelectionIntent {
  workspacePath: string;
  mode: "conversation" | "new";
  conversationId?: string | null;
}

function mergeSummary(
  previous: ConversationSummary | undefined,
  next: ConversationSummary,
): ConversationSummary {
  if (!previous) {
    return next;
  }
  return {
    ...previous,
    ...next,
    lastProviderSessionRef: next.lastProviderSessionRef ?? previous.lastProviderSessionRef,
    activeJobId: next.activeJobId ?? previous.activeJobId,
    error: next.error ?? previous.error,
  };
}

function sortConversations(items: ConversationSummary[]): ConversationSummary[] {
  return [...items].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function useConversationSession(
  workspace: WorkspaceInfo | null,
  selectionIntent: ConversationSelectionIntent | null = null,
): UseConversationSessionReturn {
  const toast = useToast();
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [activeConversation, setActiveConversation] = useState<ConversationDetail | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState<boolean>(false);
  const [sending, setSending] = useState<boolean>(false);
  const [stopping, setStopping] = useState<boolean>(false);

  useTauriEventListeners({
    listeners: [
      {
        event: CONVERSATION_EVENTS.OUTPUT_DELTA,
        handler: (payload: ConversationOutputDelta) => {
          setDrafts((previous) => ({
            ...previous,
            [payload.conversationId]: previous[payload.conversationId]
              ? `${previous[payload.conversationId]}\n${payload.text}`
              : payload.text,
          }));
        },
      },
      {
        event: CONVERSATION_EVENTS.STATUS,
        handler: (payload: ConversationStatusEvent) => {
          setConversations((previous) => sortConversations(
            upsertByKey(
              previous,
              mergeSummary(
                previous.find((item) => item.id === payload.conversation.id),
                payload.conversation,
              ),
              (item) => item.id,
            ),
          ));
          setActiveConversation((previous) => {
            if (!previous || previous.summary.id !== payload.conversation.id) {
              return previous;
            }

            return {
              summary: mergeSummary(previous.summary, payload.conversation),
              messages: payload.message ? [...previous.messages, payload.message] : previous.messages,
            };
          });
          if (payload.conversation.status !== "running") {
            setDrafts((previous) => {
              const next = { ...previous };
              delete next[payload.conversation.id];
              return next;
            });
          }
        },
      },
    ],
  });

  useEffect(() => {
    setConversations([]);
    setActiveConversation(null);
    setDrafts({});

    if (!workspace) {
      return;
    }
    const workspacePath = workspace.path;
    const selection = selectionIntent?.workspacePath === workspacePath ? selectionIntent : null;

    let cancelled = false;

    async function loadWorkspaceConversations(): Promise<void> {
      setLoading(true);
      try {
        const listed = await listWorkspaceConversations(workspacePath);
        if (cancelled) {
          return;
        }
        setConversations(sortConversations(listed));

        if (selection?.mode === "new") {
          setActiveConversation(null);
          return;
        }

        const conversationId = selection?.mode === "conversation"
          ? selection.conversationId ?? null
          : listed[0]?.id ?? null;
        if (conversationId) {
          const detail = await getConversation(workspacePath, conversationId);
          if (!cancelled) {
            setActiveConversation(detail);
          }
        } else if (!cancelled) {
          setActiveConversation(null);
        }
      } catch {
        if (!cancelled) {
          toast.error("Failed to load conversations.");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadWorkspaceConversations();

    return () => {
      cancelled = true;
    };
  }, [selectionIntent, toast, workspace]);

  const openConversation = useCallback(async (conversationId: string): Promise<void> => {
    if (!workspace) {
      return;
    }

    try {
      setLoading(true);
      const detail = await getConversation(workspace.path, conversationId);
      setActiveConversation(detail);
    } catch {
      toast.error("Failed to open conversation.");
    } finally {
      setLoading(false);
    }
  }, [toast, workspace]);

  const startNewConversation = useCallback((): void => {
    setActiveConversation(null);
  }, []);

  const sendPrompt = useCallback(async (prompt: string, agent: AgentSelection): Promise<void> => {
    if (!workspace) {
      return;
    }
    const workspacePath = workspace.path;

    setSending(true);
    try {
      if (activeConversation) {
        const updated = await sendConversationTurn(workspacePath, activeConversation.summary.id, prompt);
        setActiveConversation((previous) => previous && previous.summary.id === updated.summary.id
          ? {
              summary: mergeSummary(previous.summary, updated.summary),
              messages: updated.messages,
            }
          : updated);
        setConversations((previous) => sortConversations(
          upsertByKey(
            previous,
            mergeSummary(previous.find((item) => item.id === updated.summary.id), updated.summary),
            (item) => item.id,
          ),
        ));
        return;
      }

      const created = await createConversation(workspacePath, agent, prompt);
      const running = await sendConversationTurn(workspacePath, created.summary.id, prompt);
      setActiveConversation((previous) => previous && previous.summary.id === running.summary.id
        ? { ...previous, summary: mergeSummary(previous.summary, running.summary) }
        : running);
      setConversations((previous) => sortConversations(
        upsertByKey(
          previous,
          mergeSummary(previous.find((item) => item.id === running.summary.id), running.summary),
          (item) => item.id,
        ),
      ));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to send prompt.");
    } finally {
      setSending(false);
    }
  }, [activeConversation, toast, workspace]);

  const stopActiveConversation = useCallback(async (): Promise<void> => {
    if (!workspace || !activeConversation) {
      return;
    }
    const workspacePath = workspace.path;

    setStopping(true);
    try {
      const summary = await stopConversation(workspacePath, activeConversation.summary.id);
      setConversations((previous) => sortConversations(
        upsertByKey(
          previous,
          mergeSummary(previous.find((item) => item.id === summary.id), summary),
          (item) => item.id,
        ),
      ));
      setActiveConversation((previous) => previous ? {
        ...previous,
        summary: mergeSummary(previous.summary, summary),
      } : previous);
    } catch {
      toast.error("Failed to stop conversation.");
    } finally {
      setStopping(false);
    }
  }, [activeConversation, toast, workspace]);

  const deleteConversationById = useCallback(async (conversationId: string): Promise<void> => {
    if (!workspace) {
      return;
    }

    try {
      await deleteConversation(workspace.path, conversationId);
      setConversations((previous) => previous.filter((conversation) => conversation.id !== conversationId));
      setDrafts((previous) => {
        const next = { ...previous };
        delete next[conversationId];
        return next;
      });
      setActiveConversation((previous) => (
        previous?.summary.id === conversationId ? null : previous
      ));
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
    }
  }, [toast, workspace]);

  const activeDraft = useMemo(() => (
    activeConversation ? drafts[activeConversation.summary.id] ?? "" : ""
  ), [activeConversation, drafts]);

  return {
    conversations,
    activeConversation,
    activeDraft,
    loading,
    sending,
    stopping,
    openConversation,
    startNewConversation,
    sendPrompt,
    stopActiveConversation,
    deleteConversationById,
  };
}
