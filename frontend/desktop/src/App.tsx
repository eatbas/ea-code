import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useAppViewState } from "./hooks/useAppViewState";
import { useConversationStore } from "./hooks/useConversationStore";
import { usePrerequisites } from "./hooks/usePrerequisites";
import { useWorkspaceSession } from "./hooks/useWorkspaceSession";
import { Sidebar } from "./components/Sidebar";
import { AppContentRouter } from "./components/AppContentRouter";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import { PrerequisiteBanner } from "./components/shared/PrerequisiteBanner";
import { ProjectLoadingOverlay } from "./components/shared/ProjectLoadingOverlay";
import { openInVsCode, openProjectFolder } from "./lib/desktopApi";

function App(): ReactNode {
  const {
    workspace,
    openingWorkspace,
    openWorkspace,
    selectFolder,
    projects,
    reorderProjects,
    deleteProject,
    renameProject,
    archiveProject,
    unarchiveProject,
  } = useWorkspaceSession();
  const { status: updateStatus, updateVersion } = useUpdateCheck(false);
  const { status: prereqs, dismissed: prereqsDismissed, dismiss: dismissPrereqs } = usePrerequisites();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);

  const store = useConversationStore(projects, workspace);

  const activeProjects = useMemo(
    () => projects.filter((project) => !project.archivedAt),
    [projects],
  );

  const {
    activeView,
    setActiveView,
    handleSelectProject,
  } = useAppViewState({
    openWorkspace,
  });

  async function handleSelectProjectWithDefault(projectPath: string): Promise<void> {
    store.setConversationSelection(null);
    await handleSelectProject(projectPath);
  }

  async function handleOpenConversation(projectPath: string, conversationId: string): Promise<void> {
    store.setConversationSelection({
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
    store.setConversationSelection({
      workspacePath: projectPath,
      mode: "new",
    });
    if (workspace?.path !== projectPath) {
      await handleSelectProject(projectPath);
      return;
    }
    setActiveView("home");
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
          conversationIndex={store.conversationIndex}
          loadedProjectPaths={store.loadedProjectPaths}
          loadingProjectPaths={store.loadingProjectPaths}
          activeProjectPath={workspace?.path}
          activeConversationId={store.activeConversation?.summary.id ?? null}
          onLoadProjectConversations={store.ensureProjectConversationsLoaded}
          onSelectConversation={handleOpenConversation}
          onCreateConversation={handleCreateConversation}
          onAddProject={selectFolder}
          onReorderProjects={reorderProjects}
          onRemoveProject={(p) => { void deleteProject(p); }}
          onRenameProject={(p, name) => { void renameProject(p, name); }}
          onArchiveProject={(p) => { void archiveProject(p); }}
          onUnarchiveProject={(p) => { void unarchiveProject(p); }}
          onRemoveConversation={store.deleteConversation}
          onRenameConversation={store.renameConversation}
          onArchiveConversation={store.archiveConversation}
          onUnarchiveConversation={store.unarchiveConversation}
          onSetConversationPinned={store.setConversationPinned}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <AppContentRouter
            activeView={activeView}
            workspace={workspace}
            activeConversation={store.activeConversation}
            activeDraft={store.activeDraft}
            activePromptDraft={store.activePromptDraft}
            sendingConversation={store.sending}
            stoppingConversation={store.stopping}
            onPromptDraftChange={store.updateActivePromptDraft}
            onSendConversationPrompt={store.sendPrompt}
            onStopConversation={store.stopActiveConversation}
            projects={activeProjects}
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
