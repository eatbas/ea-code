import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import type { PipelineMode } from "../../components/ConversationView/ConversationComposer";

const PIPELINE_MODES_KEY = "maestro:pipelineModes";

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
  pipelineModes: Record<string, PipelineMode>;
  setPipelineModes: Dispatch<SetStateAction<Record<string, PipelineMode>>>;
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
  activePipelineMode: PipelineMode;
  updateActivePipelineMode: (mode: PipelineMode) => void;
  resetPipelineModeForNewConversation: (workspacePath: string) => void;
  transferPipelineModeToConversation: (workspacePath: string, conversationId: string) => void;
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
  const [pipelineModes, setPipelineModes] = useState<Record<string, PipelineMode>>(() => {
    try {
      const stored = sessionStorage.getItem(PIPELINE_MODES_KEY);
      return stored ? JSON.parse(stored) : {};
    } catch {
      return {};
    }
  });
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

  // Sync pipelineModes to sessionStorage whenever it changes.
  useEffect(() => {
    sessionStorage.setItem(PIPELINE_MODES_KEY, JSON.stringify(pipelineModes));
  }, [pipelineModes]);

  useEffect(() => {
    if (!workspace) {
      previousWorkspacePathRef.current = null;
      setConversations([]);
      setActiveConversation(null);
      setDrafts({});
      setPromptDrafts({});
      setPipelineModes({});
      stoppingConversationIdRef.current = null;
      setStoppingConversationId(null);
      return;
    }

    const workspacePath = workspace.path;
    const workspaceChanged = previousWorkspacePathRef.current !== workspacePath;
    previousWorkspacePathRef.current = workspacePath;

    const selection = selectionIntent?.workspacePath === workspacePath ? selectionIntent : null;

    if (workspaceChanged) {
      setConversations([]);
      setDrafts({});
      setActiveConversation(null);
      // Note: pipelineModes is NOT reset on workspace change — keys are scoped
      // as `${workspacePath}::${conversationId}` so no cross-workspace collision.
    } else if (selection?.mode === "new") {
      // Explicitly starting a new conversation — clear to the empty state.
      setActiveConversation(null);
    }
    // When switching between conversations in the same workspace, keep the
    // current conversation visible until the new one loads to avoid flashing
    // the empty state.
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

  const activePipelineMode: PipelineMode = useMemo(() => {
    if (!workspace) {
      return "auto";
    }
    const key = promptDraftKey(workspace.path, activeConversation?.summary.id ?? null);
    return pipelineModes[key] ?? "auto";
  }, [workspace, activeConversation, pipelineModes]);

  const updateActivePipelineMode = useCallback((mode: PipelineMode): void => {
    if (!workspace) {
      return;
    }
    const key = promptDraftKey(workspace.path, activeConversation?.summary.id ?? null);
    setPipelineModes((previous) => ({ ...previous, [key]: mode }));
  }, [workspace, activeConversation]);

  const resetPipelineModeForNewConversation = useCallback((workspacePath: string): void => {
    const key = promptDraftKey(workspacePath, null);
    setPipelineModes((previous) => ({ ...previous, [key]: "auto" }));
  }, []);

  const transferPipelineModeToConversation = useCallback((
    workspacePath: string,
    conversationId: string,
  ): void => {
    const nullKey = promptDraftKey(workspacePath, null);
    const newKey = promptDraftKey(workspacePath, conversationId);
    setPipelineModes((previous) => {
      const mode = previous[nullKey];
      if (!mode || mode === "auto") {
        return previous;
      }
      return { ...previous, [newKey]: mode };
    });
  }, []);

  return {
    conversations,
    setConversations,
    activeConversation,
    setActiveConversation,
    drafts,
    setDrafts,
    promptDrafts,
    setPromptDrafts,
    pipelineModes,
    setPipelineModes,
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
    activePipelineMode,
    updateActivePipelineMode,
    resetPipelineModeForNewConversation,
    transferPipelineModeToConversation,
  };
}
