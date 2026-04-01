import type { AgentSelection, ConversationDetail, ConversationSummary, WorkspaceInfo } from "../../types";
import { useToast } from "../../components/shared/Toast";
import { useConversationSessionActions } from "./actions";
import {
  type ConversationSelectionIntent,
  useConversationSessionState,
} from "./state";

export type { ConversationSelectionIntent } from "./state";

interface UseConversationSessionReturn {
  conversations: ConversationSummary[];
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  activePromptDraft: string;
  loading: boolean;
  sending: boolean;
  stopping: boolean;
  updateActivePromptDraft: (prompt: string) => void;
  openConversation: (conversationId: string) => Promise<void>;
  startNewConversation: () => void;
  sendPrompt: (prompt: string, agent: AgentSelection) => Promise<ConversationSummary | null>;
  stopActiveConversation: () => Promise<void>;
  deleteConversationById: (conversationId: string) => Promise<boolean>;
  renameConversationById: (conversationId: string, title: string) => Promise<ConversationSummary | null>;
  archiveConversationById: (conversationId: string) => Promise<ConversationSummary | null>;
  unarchiveConversationById: (conversationId: string) => Promise<ConversationSummary | null>;
  setConversationPinnedById: (conversationId: string, pinned: boolean) => Promise<ConversationSummary | null>;
}

export function useConversationSession(
  workspace: WorkspaceInfo | null,
  selectionIntent: ConversationSelectionIntent | null = null,
): UseConversationSessionReturn {
  const toast = useToast();
  const state = useConversationSessionState(workspace, selectionIntent, toast);
  const actions = useConversationSessionActions({
    workspace,
    toast,
    activeConversation: state.activeConversation,
    setActiveConversation: state.setActiveConversation,
    setConversations: state.setConversations,
    setDrafts: state.setDrafts,
    setPromptDrafts: state.setPromptDrafts,
    setLoading: state.setLoading,
    setSending: state.setSending,
    setStoppingConversationId: state.setStoppingConversationId,
    stoppingConversationIdRef: state.stoppingConversationIdRef,
  });

  return {
    conversations: state.conversations,
    activeConversation: state.activeConversation,
    activeDraft: state.activeDraft,
    activePromptDraft: state.activePromptDraft,
    loading: state.loading,
    sending: state.sending,
    stopping: state.activeConversation?.summary.id === state.stoppingConversationId,
    updateActivePromptDraft: state.updateActivePromptDraft,
    ...actions,
  };
}
