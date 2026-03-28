import { useState } from "react";
import type {
  AgentSelection,
  ConversationDetail,
  ConversationSummary,
  ProjectEntry,
  WorkspaceInfo,
} from "../types";
import {
  archiveConversation as archiveConversationApi,
  deleteConversation as deleteConversationApi,
  renameConversation as renameConversationApi,
  setConversationPinned as setConversationPinnedApi,
  unarchiveConversation as unarchiveConversationApi,
} from "../lib/desktopApi";
import {
  useConversationSession,
  type ConversationSelectionIntent,
} from "./useConversationSession";
import { useProjectConversationIndex } from "./useProjectConversationIndex";
import { useToast } from "../components/shared/Toast";

interface UseConversationStoreReturn {
  /** Active conversation detail for the current workspace. */
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  activePromptDraft: string;
  sending: boolean;
  stopping: boolean;
  updateActivePromptDraft: (prompt: string) => void;
  sendPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  stopActiveConversation: () => Promise<void>;

  /** Sidebar conversation index keyed by project path. */
  conversationIndex: Record<string, ConversationSummary[]>;
  loadedProjectPaths: Set<string>;
  loadingProjectPaths: Set<string>;
  ensureProjectConversationsLoaded: (projectPath: string) => Promise<void>;

  /** Unified CRUD — works for any project, bridges session + index internally. */
  deleteConversation: (projectPath: string, conversationId: string) => void;
  renameConversation: (projectPath: string, conversationId: string, title: string) => void;
  archiveConversation: (projectPath: string, conversationId: string) => void;
  unarchiveConversation: (projectPath: string, conversationId: string) => void;
  setConversationPinned: (projectPath: string, conversationId: string, pinned: boolean) => void;

  /** Selection intent state for App-level routing. */
  conversationSelection: ConversationSelectionIntent | null;
  setConversationSelection: (intent: ConversationSelectionIntent | null) => void;
}

/**
 * Unified conversation store that composes `useConversationSession` (active
 * workspace detail + streaming) with `useProjectConversationIndex` (sidebar
 * summaries for all projects).
 *
 * All CRUD operations are handled here — callers no longer need to decide
 * which hook to call or manually sync results between the two stores.
 */
export function useConversationStore(
  projects: ProjectEntry[],
  workspace: WorkspaceInfo | null,
): UseConversationStoreReturn {
  const toast = useToast();
  const [conversationSelection, setConversationSelection] =
    useState<ConversationSelectionIntent | null>(null);

  const {
    activeConversation,
    activeDraft,
    activePromptDraft,
    sending,
    stopping,
    updateActivePromptDraft,
    sendPrompt,
    stopActiveConversation,
    deleteConversationById,
    renameConversationById,
    archiveConversationById,
    unarchiveConversationById,
    setConversationPinnedById,
  } = useConversationSession(workspace, conversationSelection);

  const {
    index: conversationIndex,
    loadedProjectPaths,
    loadingProjectPaths,
    ensureLoaded: ensureProjectConversationsLoaded,
    upsertConversation: upsertInIndex,
    removeConversation: removeFromIndex,
  } = useProjectConversationIndex(projects);

  const isActiveWorkspace = (projectPath: string): boolean =>
    workspace?.path === projectPath;

  /** Run a CRUD action that returns a summary, syncing both stores. */
  function upsertAction(
    projectPath: string,
    sessionFn: () => Promise<ConversationSummary | null>,
    directFn: () => Promise<ConversationSummary>,
    errorLabel: string,
  ): void {
    void (async () => {
      try {
        const summary = isActiveWorkspace(projectPath)
          ? await sessionFn()
          : await directFn();
        if (summary) {
          upsertInIndex(summary);
        }
      } catch (error) {
        toast.error(error instanceof Error ? error.message : errorLabel);
      }
    })();
  }

  return {
    activeConversation,
    activeDraft,
    activePromptDraft,
    sending,
    stopping,
    updateActivePromptDraft,
    sendPrompt,
    stopActiveConversation,

    conversationIndex,
    loadedProjectPaths,
    loadingProjectPaths,
    ensureProjectConversationsLoaded,

    deleteConversation: (projectPath, conversationId) => {
      void (async () => {
        try {
          if (isActiveWorkspace(projectPath)) {
            const deleted = await deleteConversationById(conversationId);
            if (deleted) removeFromIndex(projectPath, conversationId);
          } else {
            await deleteConversationApi(projectPath, conversationId);
            removeFromIndex(projectPath, conversationId);
          }
        } catch (error) {
          toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
        }
      })();
    },

    renameConversation: (projectPath, conversationId, title) =>
      upsertAction(projectPath,
        () => renameConversationById(conversationId, title),
        () => renameConversationApi(projectPath, conversationId, title),
        "Failed to rename conversation."),

    archiveConversation: (projectPath, conversationId) =>
      upsertAction(projectPath,
        () => archiveConversationById(conversationId),
        () => archiveConversationApi(projectPath, conversationId),
        "Failed to archive conversation."),

    unarchiveConversation: (projectPath, conversationId) =>
      upsertAction(projectPath,
        () => unarchiveConversationById(conversationId),
        () => unarchiveConversationApi(projectPath, conversationId),
        "Failed to unarchive conversation."),

    setConversationPinned: (projectPath, conversationId, pinned) =>
      upsertAction(projectPath,
        () => setConversationPinnedById(conversationId, pinned),
        () => setConversationPinnedApi(projectPath, conversationId, pinned),
        "Failed to update pin."),

    conversationSelection,
    setConversationSelection,
  };
}
