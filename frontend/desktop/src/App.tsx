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
import { deleteConversation, openInVsCode, openProjectFolder } from "./lib/desktopApi";
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
  } = useWorkspaceSession();
  const { status: updateStatus, updateVersion } = useUpdateCheck(false);
  const { status: prereqs, dismissed: prereqsDismissed, dismiss: dismissPrereqs } = usePrerequisites();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [conversationSelection, setConversationSelection] = useState<ConversationSelectionIntent | null>(null);
  const {
    activeConversation,
    activeDraft,
    sending,
    stopping,
    sendPrompt,
    stopActiveConversation,
    deleteConversationById,
  } = useConversationSession(workspace, conversationSelection);
  const {
    index: conversationIndex,
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
      if (workspace?.path === projectPath) {
        await deleteConversationById(conversationId);
      } else {
        await deleteConversation(projectPath, conversationId);
      }
      removeConversationFromIndex(projectPath, conversationId);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : "Failed to delete conversation.");
    }
  }

  return (
    <div className="relative h-full">
      <div className={`flex h-full bg-[#0b0b0c] transition-[filter] duration-200 ${openingWorkspace ? "pointer-events-none blur-[2px]" : ""}`}>
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
          onRemoveConversation={(projectPath, conversationId) => {
            void handleDeleteConversation(projectPath, conversationId);
          }}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <AppContentRouter
            activeView={activeView}
            workspace={workspace}
            activeConversation={activeConversation}
            activeDraft={activeDraft}
            sendingConversation={sending}
            stoppingConversation={stopping}
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
