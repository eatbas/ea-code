import type { ReactNode } from "react";
import { useState } from "react";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useAppViewState } from "./hooks/useAppViewState";
import {
  useConversationSession,
  type ConversationSelectionIntent,
} from "./hooks/useConversationSession";
import { useProjectConversationIndex } from "./hooks/useProjectConversationIndex";
import { usePrerequisites } from "./hooks/usePrerequisites";
import { useWorkspaceSession } from "./hooks/useWorkspaceSession";
import { Sidebar } from "./components/Sidebar";
import { AppContentRouter } from "./components/AppContentRouter";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import { PrerequisiteBanner } from "./components/shared/PrerequisiteBanner";
import { ProjectLoadingOverlay } from "./components/shared/ProjectLoadingOverlay";
import {
  archiveConversation,
  deleteConversation,
  openInVsCode,
  openProjectFolder,
  renameConversation,
  setConversationPinned,
} from "./lib/desktopApi";
import { useToast } from "./components/shared/Toast";

function App(): ReactNode {
  const toast = useToast();
  const {
    workspace,
    openingWorkspace,
    openWorkspace,
    selectFolder,
    projects,
    deleteProject,
    renameProject,
    archiveProject,
  } = useWorkspaceSession();
  const { status: updateStatus, updateVersion } = useUpdateCheck(false);
  const { status: prereqs, dismissed: prereqsDismissed, dismiss: dismissPrereqs } = usePrerequisites();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [conversationSelection, setConversationSelection] = useState<ConversationSelectionIntent | null>(null);
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
    setConversationPinnedById,
  } = useConversationSession(workspace, conversationSelection);
  const {
    index: conversationIndex,
    upsertConversation: upsertConversationInIndex,
    removeConversation: removeConversationFromIndex,
  } = useProjectConversationIndex(projects);

  const {
    activeView,
    setActiveView,
    handleSelectProject,
  } = useAppViewState({
    openWorkspace,
  });

  async function handleSelectProjectWithDefault(projectPath: string): Promise<void> {
    setConversationSelection(null);
    await handleSelectProject(projectPath);
  }

  async function handleOpenConversation(projectPath: string, conversationId: string): Promise<void> {
    setConversationSelection({
      workspacePath: projectPath,
      mode: "conversation",
      conversationId,
    });
    if (workspace?.path !== projectPath) {
      await handleSelectProject(projectPath);
      return;
    }
    setActiveView("home");
  }

  async function handleCreateConversation(projectPath: string): Promise<void> {
    setConversationSelection({
      workspacePath: projectPath,
      mode: "new",
    });
    if (workspace?.path !== projectPath) {
      await handleSelectProject(projectPath);
      return;
    }
    setActiveView("home");
  }

  async function handleDeleteConversation(projectPath: string, conversationId: string): Promise<void> {
    try {
      let deleted = false;
      if (workspace?.path === projectPath) {
        deleted = await deleteConversationById(conversationId);
      } else {
        await deleteConversation(projectPath, conversationId);
        deleted = true;
      }
      if (deleted) {
        removeConversationFromIndex(projectPath, conversationId);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
    }
  }

  async function handleRenameConversation(
    projectPath: string,
    conversationId: string,
    title: string,
  ): Promise<void> {
    try {
      const summary = workspace?.path === projectPath
        ? await renameConversationById(conversationId, title)
        : await renameConversation(projectPath, conversationId, title);
      if (summary) {
        upsertConversationInIndex(summary);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to rename conversation.");
    }
  }

  async function handleArchiveConversation(projectPath: string, conversationId: string): Promise<void> {
    try {
      const summary = workspace?.path === projectPath
        ? await archiveConversationById(conversationId)
        : await archiveConversation(projectPath, conversationId);
      if (summary) {
        removeConversationFromIndex(projectPath, conversationId);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to archive conversation.");
    }
  }

  async function handleSetConversationPinned(
    projectPath: string,
    conversationId: string,
    pinned: boolean,
  ): Promise<void> {
    try {
      const summary = workspace?.path === projectPath
        ? await setConversationPinnedById(conversationId, pinned)
        : await setConversationPinned(projectPath, conversationId, pinned);
      if (summary) {
        upsertConversationInIndex(summary);
      }
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to update pin.");
    }
  }

  return (
    <div className="relative h-full">
      <div className={`flex h-full bg-surface transition-[filter] duration-200 ${openingWorkspace ? "pointer-events-none blur-[2px]" : ""}`}>
        <Sidebar
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed((prev) => !prev)}
          activeView={activeView}
          onNavigate={setActiveView}
          projects={projects}
          conversationIndex={conversationIndex}
          activeProjectPath={workspace?.path}
          activeConversationId={activeConversation?.summary.id ?? null}
          onSelectProject={handleSelectProjectWithDefault}
          onSelectConversation={handleOpenConversation}
          onCreateConversation={handleCreateConversation}
          onAddProject={selectFolder}
          onRemoveProject={(projectPath) => {
            void deleteProject(projectPath);
          }}
          onRenameProject={(projectPath, name) => {
            void renameProject(projectPath, name);
          }}
          onArchiveProject={(projectPath) => {
            void archiveProject(projectPath);
          }}
          onRemoveConversation={(projectPath, conversationId) => {
            void handleDeleteConversation(projectPath, conversationId);
          }}
          onRenameConversation={(projectPath, conversationId, title) => {
            void handleRenameConversation(projectPath, conversationId, title);
          }}
          onArchiveConversation={(projectPath, conversationId) => {
            void handleArchiveConversation(projectPath, conversationId);
          }}
          onSetConversationPinned={(projectPath, conversationId, pinned) => {
            void handleSetConversationPinned(projectPath, conversationId, pinned);
          }}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <AppContentRouter
            activeView={activeView}
            workspace={workspace}
            activeConversation={activeConversation}
            activeDraft={activeDraft}
            activePromptDraft={activePromptDraft}
            sendingConversation={sending}
            stoppingConversation={stopping}
            onPromptDraftChange={updateActivePromptDraft}
            onSendConversationPrompt={sendPrompt}
            onStopConversation={stopActiveConversation}
            projects={projects}
            onSelectProject={handleSelectProjectWithDefault}
            onAddProject={selectFolder}
            onOpenProjectFolder={openProjectFolder}
            onOpenInVsCode={openInVsCode}
          />
        </div>
        {updateStatus !== "idle" && (
          <UpdateInstallBanner
            mode={updateStatus}
            version={updateVersion}
          />
        )}
      </div>
      {prereqs && !prereqsDismissed && (!prereqs.pythonAvailable || !prereqs.gitBashAvailable) && (
        <PrerequisiteBanner status={prereqs} onDismiss={dismissPrereqs} />
      )}
      {openingWorkspace && <ProjectLoadingOverlay />}
    </div>
  );
}

export default App;
