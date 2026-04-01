import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import type {
  ConversationDetail,
  ConversationOutputDelta,
  ConversationStatusEvent,
  ConversationSummary,
  WorkspaceInfo,
} from "../../types";
import { CONVERSATION_EVENTS } from "../../constants/events";
import {
  getConversation,
  listWorkspaceConversations,
} from "../../lib/desktopApi";
import { useTauriEventListeners } from "../useTauriEventListeners";
import {
  mergeSummary,
  promptDraftKey,
  removeEntry,
  sortConversations,
  upsertConversationSummary,
} from "./helpers";

export interface ConversationSelectionIntent {
  workspacePath: string;
  mode: "conversation" | "new";
  conversationId?: string | null;
}

interface ToastApi {
  error: (message: string) => void;
}

export interface UseConversationSessionState {
  conversations: ConversationSummary[];
  setConversations: Dispatch<SetStateAction<ConversationSummary[]>>;
  activeConversation: ConversationDetail | null;
  setActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  drafts: Record<string, string>;
  setDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  promptDrafts: Record<string, string>;
  setPromptDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  loading: boolean;
  setLoading: Dispatch<SetStateAction<boolean>>;
  sending: boolean;
  setSending: Dispatch<SetStateAction<boolean>>;
  stoppingConversationId: string | null;
  setStoppingConversationId: Dispatch<SetStateAction<string | null>>;
  stoppingConversationIdRef: MutableRefObject<string | null>;
  activeDraft: string;
  activePromptDraft: string;
  updateActivePromptDraft: (prompt: string) => void;
}

export function useConversationSessionState(
  workspace: WorkspaceInfo | null,
  selectionIntent: ConversationSelectionIntent | null,
  toast: ToastApi,
): UseConversationSessionState {
  const [conversations, setConversations] = useState<ConversationSummary[]>([]);
  const [activeConversation, setActiveConversation] = useState<ConversationDetail | null>(null);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [promptDrafts, setPromptDrafts] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState<boolean>(false);
  const [sending, setSending] = useState<boolean>(false);
  const [stoppingConversationId, setStoppingConversationId] = useState<string | null>(null);
  const previousWorkspacePathRef = useRef<string | null>(null);
  const stoppingConversationIdRef = useRef<string | null>(null);
  const selectionWorkspacePath = selectionIntent?.workspacePath ?? null;
  const selectionMode = selectionIntent?.mode ?? null;
  const selectionConversationId = selectionIntent?.conversationId ?? null;

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
          setConversations((previous) => upsertConversationSummary(previous, payload.conversation));
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
            setDrafts((previous) => removeEntry(previous, payload.conversation.id));
            if (stoppingConversationIdRef.current === payload.conversation.id) {
              stoppingConversationIdRef.current = null;
              setStoppingConversationId(null);
            }
          }
        },
      },
    ],
  });

  useEffect(() => {
    if (!workspace) {
      previousWorkspacePathRef.current = null;
      setConversations([]);
      setActiveConversation(null);
      setDrafts({});
      setPromptDrafts({});
      stoppingConversationIdRef.current = null;
      setStoppingConversationId(null);
      return;
    }

    const workspacePath = workspace.path;
    const workspaceChanged = previousWorkspacePathRef.current !== workspacePath;
    previousWorkspacePathRef.current = workspacePath;

    if (workspaceChanged) {
      setConversations([]);
      setDrafts({});
    }
    setActiveConversation(null);

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

        if (!conversationId) {
          setActiveConversation(null);
          return;
        }

        const detail = await getConversation(workspacePath, conversationId);
        if (!cancelled) {
          setActiveConversation(detail);
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
  }, [selectionConversationId, selectionMode, selectionWorkspacePath, toast, workspace]);

  const activeDraft = useMemo(
    () => (activeConversation ? drafts[activeConversation.summary.id] ?? "" : ""),
    [activeConversation, drafts],
  );

  const activePromptDraft = useMemo(() => {
    if (!workspace) {
      return "";
    }

    return promptDrafts[promptDraftKey(workspace.path, activeConversation?.summary.id ?? null)] ?? "";
  }, [activeConversation, promptDrafts, workspace]);

  const updateActivePromptDraft = (prompt: string): void => {
    if (!workspace) {
      return;
    }

    const key = promptDraftKey(workspace.path, activeConversation?.summary.id ?? null);
    setPromptDrafts((previous) => ({
      ...previous,
      [key]: prompt,
    }));
  };

  return {
    conversations,
    setConversations,
    activeConversation,
    setActiveConversation,
    drafts,
    setDrafts,
    promptDrafts,
    setPromptDrafts,
    loading,
    setLoading,
    sending,
    setSending,
    stoppingConversationId,
    setStoppingConversationId,
    stoppingConversationIdRef,
    activeDraft,
    activePromptDraft,
    updateActivePromptDraft,
  };
}
