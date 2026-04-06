import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useCallback } from "react";
import type { AgentSelection, ConversationDetail, ConversationSummary, WorkspaceInfo } from "../../types";
import {
  archiveConversation,
  createConversation,
  deleteConversation,
  getConversation,
  renameConversation,
  sendConversationTurn,
  setConversationPinned,
  stopConversation,
  unarchiveConversation,
} from "../../lib/desktopApi";
import {
  promptDraftKey,
  removeEntry,
  updateActiveConversationSummary,
  upsertConversationSummary,
} from "./helpers";

interface ToastApi {
  error: (message: string) => void;
}

interface UseConversationSessionActionParams {
  workspace: WorkspaceInfo | null;
  toast: ToastApi;
  activeConversation: ConversationDetail | null;
  setActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  setConversations: Dispatch<SetStateAction<ConversationSummary[]>>;
  setDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  setPromptDrafts: Dispatch<SetStateAction<Record<string, string>>>;
  setLoading: Dispatch<SetStateAction<boolean>>;
  setSending: Dispatch<SetStateAction<boolean>>;
  setStoppingConversationId: Dispatch<SetStateAction<string | null>>;
  stoppingConversationIdRef: MutableRefObject<string | null>;
  transferPipelineModeToConversation: (workspacePath: string, conversationId: string) => void;
}

export function useConversationSessionActions({
  workspace,
  toast,
  activeConversation,
  setActiveConversation,
  setConversations,
  setDrafts,
  setPromptDrafts,
  setLoading,
  setSending,
  setStoppingConversationId,
  stoppingConversationIdRef,
  transferPipelineModeToConversation,
}: UseConversationSessionActionParams) {
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
  }, [setActiveConversation, setLoading, toast, workspace]);

  const startNewConversation = useCallback((): void => {
    setActiveConversation(null);
  }, [setActiveConversation]);

  const sendPrompt = useCallback(async (
    prompt: string,
    agent: AgentSelection,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    const workspacePath = workspace.path;
    setSending(true);

    try {
      if (activeConversation) {
        const updated = await sendConversationTurn(workspacePath, activeConversation.summary.id, prompt);
        setPromptDrafts((previous) => ({
          ...previous,
          [promptDraftKey(workspacePath, activeConversation.summary.id)]: "",
        }));
        setActiveConversation((previous) => {
          if (previous?.summary.id !== updated.summary.id) {
            return updated;
          }

          return {
            summary: {
              ...previous.summary,
              ...updated.summary,
            },
            messages: updated.messages,
          };
        });
        setConversations((previous) => upsertConversationSummary(previous, updated.summary));
        return updated.summary;
      }

      const created = await createConversation(workspacePath, agent, prompt);
      transferPipelineModeToConversation(workspacePath, created.summary.id);
      const running = await sendConversationTurn(workspacePath, created.summary.id, prompt);
      setPromptDrafts((previous) => ({
        ...previous,
        [promptDraftKey(workspacePath, null)]: "",
        [promptDraftKey(workspacePath, created.summary.id)]: "",
      }));
      setActiveConversation((previous) => (
        previous?.summary.id === running.summary.id
          ? { ...previous, summary: { ...previous.summary, ...running.summary } }
          : running
      ));
      setConversations((previous) => upsertConversationSummary(previous, running.summary));
      return running.summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to send prompt.");
      return null;
    } finally {
      setSending(false);
    }
  }, [activeConversation, setActiveConversation, setConversations, setPromptDrafts, setSending, toast, transferPipelineModeToConversation, workspace]);

  const stopActiveConversation = useCallback(async (): Promise<void> => {
    if (!workspace || !activeConversation) {
      return;
    }

    const workspacePath = workspace.path;
    const conversationId = activeConversation.summary.id;
    stoppingConversationIdRef.current = conversationId;
    setStoppingConversationId(conversationId);

    try {
      const summary = await stopConversation(workspacePath, conversationId);
      setConversations((previous) => upsertConversationSummary(previous, summary));
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      if (summary.status !== "running") {
        stoppingConversationIdRef.current = null;
        setStoppingConversationId(null);
      }
    } catch (error) {
      stoppingConversationIdRef.current = null;
      setStoppingConversationId(null);
      toast.error(error instanceof Error ? error.message : "Failed to stop conversation.");
    }
  }, [
    activeConversation,
    setActiveConversation,
    setConversations,
    setStoppingConversationId,
    stoppingConversationIdRef,
    toast,
    workspace,
  ]);

  const deleteConversationById = useCallback(async (conversationId: string): Promise<boolean> => {
    if (!workspace) {
      return false;
    }

    try {
      await deleteConversation(workspace.path, conversationId);
      setConversations((previous) => previous.filter((conversation) => conversation.id !== conversationId));
      setDrafts((previous) => removeEntry(previous, conversationId));
      setPromptDrafts((previous) => removeEntry(previous, promptDraftKey(workspace.path, conversationId)));
      setActiveConversation((previous) => (
        previous?.summary.id === conversationId ? null : previous
      ));
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
      return false;
    }
  }, [setActiveConversation, setConversations, setDrafts, setPromptDrafts, toast, workspace]);

  const renameConversationById = useCallback(async (
    conversationId: string,
    title: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await renameConversation(workspace.path, conversationId, title);
      setConversations((previous) => upsertConversationSummary(previous, summary));
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to rename conversation.");
      return null;
    }
  }, [setActiveConversation, setConversations, toast, workspace]);

  const archiveConversationById = useCallback(async (
    conversationId: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await archiveConversation(workspace.path, conversationId);
      setConversations((previous) => previous.filter((conversation) => conversation.id !== conversationId));
      setDrafts((previous) => removeEntry(previous, conversationId));
      setPromptDrafts((previous) => removeEntry(previous, promptDraftKey(workspace.path, conversationId)));
      setActiveConversation((previous) => (
        previous?.summary.id === conversationId ? null : previous
      ));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to archive conversation.");
      return null;
    }
  }, [setActiveConversation, setConversations, setDrafts, setPromptDrafts, toast, workspace]);

  const unarchiveConversationById = useCallback(async (
    conversationId: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await unarchiveConversation(workspace.path, conversationId);
      setConversations((previous) => upsertConversationSummary(previous, summary));
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to unarchive conversation.");
      return null;
    }
  }, [setActiveConversation, setConversations, toast, workspace]);

  const setConversationPinnedById = useCallback(async (
    conversationId: string,
    pinned: boolean,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await setConversationPinned(workspace.path, conversationId, pinned);
      setConversations((previous) => upsertConversationSummary(previous, summary));
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to update pin.");
      return null;
    }
  }, [setActiveConversation, setConversations, toast, workspace]);

  return {
    openConversation,
    startNewConversation,
    sendPrompt,
    stopActiveConversation,
    deleteConversationById,
    renameConversationById,
    archiveConversationById,
    unarchiveConversationById,
    setConversationPinnedById,
  };
}
