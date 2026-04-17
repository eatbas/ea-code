import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { useCallback } from "react";
import type { AgentSelection, ConversationDetail, ConversationSummary, WorkspaceInfo } from "../../types";
import type { PendingImage } from "../useImageAttachments";
import {
  archiveConversation,
  createConversation,
  deleteConversation,
  getConversation,
  renameConversation,
  saveConversationImage,
  sendConversationTurn,
  setConversationPinned,
  stopConversation,
  unarchiveConversation,
} from "../../lib/desktopApi";
import {
  promptDraftKey,
  removeEntry,
  updateActiveConversationSummary,
} from "./helpers";
import { blobToBase64, buildPromptWithImages } from "../../utils/imageUtils";

async function flushAndSaveImages(
  workspacePath: string,
  conversationId: string,
  pendingImages: PendingImage[],
): Promise<string[]> {
  const paths: string[] = [];
  for (const pending of pendingImages) {
    try {
      const base64 = await blobToBase64(pending.blob);
      const result = await saveConversationImage(workspacePath, conversationId, base64, pending.extension);
      paths.push(result.filePath);
    } catch (error) {
      console.warn("[sendPrompt] Failed to save pending image:", error);
    }
  }
  return paths;
}

interface ToastApi {
  error: (message: string) => void;
}

interface UseConversationSessionActionParams {
  workspace: WorkspaceInfo | null;
  toast: ToastApi;
  activeConversation: ConversationDetail | null;
  setActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
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
    pendingImages?: PendingImage[],
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    const workspacePath = workspace.path;
    setSending(true);

    try {
      if (activeConversation) {
        let finalPrompt = prompt;
        if (pendingImages && pendingImages.length > 0) {
          const paths = await flushAndSaveImages(workspacePath, activeConversation.summary.id, pendingImages);
          finalPrompt = buildPromptWithImages(prompt, paths);
        }
        // Forward model override when the user switched model on a resumed conversation.
        const modelChanged = agent.model !== activeConversation.summary.agent.model;
        const updated = await sendConversationTurn(
          workspacePath,
          activeConversation.summary.id,
          finalPrompt,
          modelChanged ? agent.model : undefined,
        );
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
        return updated.summary;
      }

      const created = await createConversation(workspacePath, agent, prompt);
      transferPipelineModeToConversation(workspacePath, created.summary.id);

      let finalPrompt = prompt;
      if (pendingImages && pendingImages.length > 0) {
        const paths = await flushAndSaveImages(workspacePath, created.summary.id, pendingImages);
        finalPrompt = buildPromptWithImages(prompt, paths);
      }

      const running = await sendConversationTurn(workspacePath, created.summary.id, finalPrompt);
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
      return running.summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to send prompt.");
      return null;
    } finally {
      setSending(false);
    }
  }, [activeConversation, setActiveConversation, setPromptDrafts, setSending, toast, transferPipelineModeToConversation, workspace]);

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
      setPromptDrafts((previous) => removeEntry(previous, promptDraftKey(workspace.path, conversationId)));
      // Active detail + drafts are cleared by the DELETED event listener.
      return true;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
      return false;
    }
  }, [setPromptDrafts, toast, workspace]);

  const renameConversationById = useCallback(async (
    conversationId: string,
    title: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await renameConversation(workspace.path, conversationId, title);
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to rename conversation.");
      return null;
    }
  }, [setActiveConversation, toast, workspace]);

  const archiveConversationById = useCallback(async (
    conversationId: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await archiveConversation(workspace.path, conversationId);
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
  }, [setActiveConversation, setDrafts, setPromptDrafts, toast, workspace]);

  const unarchiveConversationById = useCallback(async (
    conversationId: string,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await unarchiveConversation(workspace.path, conversationId);
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to unarchive conversation.");
      return null;
    }
  }, [setActiveConversation, toast, workspace]);

  const setConversationPinnedById = useCallback(async (
    conversationId: string,
    pinned: boolean,
  ): Promise<ConversationSummary | null> => {
    if (!workspace) {
      return null;
    }

    try {
      const summary = await setConversationPinned(workspace.path, conversationId, pinned);
      setActiveConversation((previous) => updateActiveConversationSummary(previous, summary));
      return summary;
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to update pin.");
      return null;
    }
  }, [setActiveConversation, toast, workspace]);

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
