import type { Dispatch, SetStateAction } from "react";
import { useState } from "react";
import type {
  AgentSelection,
  ConversationDetail,
  ConversationSummary,
  ProjectEntry,
  WorkspaceInfo,
} from "../types";
import type { PendingImage } from "./useImageAttachments";
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
import type { PipelineMode } from "../components/ConversationView/ConversationComposer";

/**
 * Backend CRUD commands emit `CONVERSATION_EVENTS.STATUS` (and
 * `CONVERSATION_EVENTS.DELETED` for removals) after they complete. Both
 * stores — the active-workspace session and the cross-project index —
 * subscribe to those events directly, so callers only need to invoke the
 * backend; no manual fan-out is required here.
 */

interface UseConversationStoreReturn {
  /** Active conversation detail for the current workspace. */
  activeConversation: ConversationDetail | null;
  /** Set the active conversation directly (e.g. after starting a pipeline). */
  setActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  activeDraft: string;
  activePromptDraft: string;
  activePipelineMode: PipelineMode;
  updateActivePipelineMode: (mode: PipelineMode) => void;
  resetPipelineModeForNewConversation: (workspacePath: string) => void;
  sending: boolean;
  stopping: boolean;
  updateActivePromptDraft: (prompt: string) => void;
  sendPrompt: (prompt: string, agent: AgentSelection, pendingImages?: PendingImage[]) => Promise<void>;
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

function sameSelectionIntent(
  left: ConversationSelectionIntent | null,
  right: ConversationSelectionIntent | null,
): boolean {
  if (left === right) {
    return true;
  }
  if (!left || !right) {
    return false;
  }

  return left.workspacePath === right.workspacePath
    && left.mode === right.mode
    && (left.conversationId ?? null) === (right.conversationId ?? null);
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
  const [conversationSelection, setConversationSelectionState] =
    useState<ConversationSelectionIntent | null>(null);

  const {
    activeConversation,
    setActiveConversation,
    activeDraft,
    activePromptDraft,
    activePipelineMode,
    updateActivePipelineMode,
    resetPipelineModeForNewConversation,
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
  } = useProjectConversationIndex(projects);

  const isActiveWorkspace = (projectPath: string): boolean =>
    workspace?.path === projectPath;

  /** Run a CRUD action; the index/session update themselves via STATUS events. */
  function upsertAction(
    projectPath: string,
    sessionFn: () => Promise<ConversationSummary | null>,
    directFn: () => Promise<ConversationSummary>,
    errorLabel: string,
  ): void {
    void (async () => {
      try {
        if (isActiveWorkspace(projectPath)) {
          await sessionFn();
        } else {
          await directFn();
        }
      } catch (error) {
        toast.error(error instanceof Error ? error.message : errorLabel);
      }
    })();
  }

  return {
    activeConversation,
    setActiveConversation,
    activeDraft,
    activePromptDraft,
    activePipelineMode,
    updateActivePipelineMode,
    resetPipelineModeForNewConversation,
    sending,
    stopping,
    updateActivePromptDraft,
    sendPrompt: async (prompt: string, agent: AgentSelection, pendingImages?: PendingImage[]): Promise<void> => {
      await sendPrompt(prompt, agent, pendingImages);
    },
    stopActiveConversation,

    conversationIndex,
    loadedProjectPaths,
    loadingProjectPaths,
    ensureProjectConversationsLoaded,

    deleteConversation: (projectPath, conversationId) => {
      void (async () => {
        try {
          if (isActiveWorkspace(projectPath)) {
            await deleteConversationById(conversationId);
          } else {
            await deleteConversationApi(projectPath, conversationId);
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
    setConversationSelection: (intent) => {
      setConversationSelectionState((previous) => (
        sameSelectionIntent(previous, intent) ? previous : intent
      ));
    },
  };
}
