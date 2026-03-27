import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { useCliHealth } from "./hooks/useCliHealth";
import { useApiHealth } from "./hooks/useApiHealth";
import { useApiCliVersions } from "./hooks/useApiCliVersions";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useAppViewState } from "./hooks/useAppViewState";
import { usePrerequisites } from "./hooks/usePrerequisites";
import { Sidebar } from "./components/Sidebar";
import { AppContentRouter } from "./components/AppContentRouter";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import { PrerequisiteBanner } from "./components/shared/PrerequisiteBanner";
import { ProjectLoadingOverlay } from "./components/shared/ProjectLoadingOverlay";

function App(): ReactNode {
  const { workspace, openingWorkspace, openWorkspace, selectFolder, projects, loadProjects, deleteProject } = useWorkspace();
  const { settings, loading, saveSettings } = useSettings();
  const { checkHealth } = useCliHealth();
  const { health: apiHealth, providers, checking: providersLoading, checkHealth: checkApiHealth } = useApiHealth();
  const { versions: apiVersions, loading: apiVersionsLoading, updating: apiVersionsUpdating, fetchVersions: fetchApiVersions, updateCli: updateApiCli } = useApiCliVersions();
  const { status: updateStatus, updateVersion } = useUpdateCheck(false);
  const { status: prereqs, dismissed: prereqsDismissed, dismiss: dismissPrereqs } = usePrerequisites();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);

  const {
    activeView,
    setActiveView,
    handleSelectProject,
  } = useAppViewState({
    workspace,
    openWorkspace,
  });

  useEffect(() => {
    if (!settings) return;
    checkHealth(settings);
    checkApiHealth();
  }, [settings, checkHealth, checkApiHealth]);

  useEffect(() => {
    void loadProjects();
  }, [loadProjects]);

  useEffect(() => {
    if (workspace) {
      void loadProjects();
    }
  }, [workspace, loadProjects]);

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Loading...</span>
      </div>
    );
  }

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
            providers={providers}
            providersLoading={providersLoading}
            apiVersions={apiVersions}
            apiVersionsLoading={apiVersionsLoading}
            apiVersionsUpdating={apiVersionsUpdating}
            apiHealth={apiHealth}
            settings={settings}
            onSaveSettings={saveSettings}
            onFetchApiVersions={fetchApiVersions}
            onRefreshProviders={checkApiHealth}
            onUpdateApiCli={updateApiCli}
            onSelectProject={handleSelectProject}
            onAddProject={selectFolder}
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
