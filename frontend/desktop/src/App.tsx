import type { ReactNode } from "react";
import { useState } from "react";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useAppViewState } from "./hooks/useAppViewState";
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
    deleteProject,
  } = useWorkspaceSession();
  const { status: updateStatus, updateVersion } = useUpdateCheck(false);
  const { status: prereqs, dismissed: prereqsDismissed, dismiss: dismissPrereqs } = usePrerequisites();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);

  const {
    activeView,
    setActiveView,
    handleSelectProject,
  } = useAppViewState({
    openWorkspace,
  });

  return (
    <div className="relative h-full">
      <div className={`flex h-full bg-[#0f0f14] transition-[filter] duration-200 ${openingWorkspace ? "pointer-events-none blur-[2px]" : ""}`}>
        <Sidebar
          collapsed={sidebarCollapsed}
          onToggle={() => setSidebarCollapsed((prev) => !prev)}
          activeView={activeView}
          onNavigate={setActiveView}
          projects={projects}
          activeProjectPath={workspace?.path}
          onSelectProject={handleSelectProject}
          onAddProject={selectFolder}
          onRemoveProject={(projectPath) => {
            void deleteProject(projectPath);
          }}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <AppContentRouter
            activeView={activeView}
            workspace={workspace}
            projects={projects}
            onSelectProject={handleSelectProject}
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
