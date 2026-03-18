import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { useToast } from "./components/shared/Toast";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliVersions } from "./hooks/useCliVersions";
import { useCliHealth } from "./hooks/useCliHealth";
import { useHistory } from "./hooks/useHistory";
import { useSkills } from "./hooks/useSkills";
import { useLiveSessionStatus } from "./hooks/useLiveSessionStatus";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { useAppViewState } from "./hooks/useAppViewState";
import { isActive } from "./utils/statusHelpers";
import { Sidebar } from "./components/Sidebar";
import { AppContentRouter } from "./components/AppContentRouter";
import { QuestionDialog } from "./components/QuestionDialog";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import { ProjectLoadingOverlay } from "./components/shared/ProjectLoadingOverlay";

function App(): ReactNode {
  const toast = useToast();
  const { workspace, openingWorkspace, openWorkspace, selectFolder } = useWorkspace();
  const { settings, loading, saveSettings } = useSettings();
  const { run, stageLogs, artifacts, pendingQuestion, startPipeline, pausePipeline, resumePipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { versions, loading: versionsLoading, updating: versionsUpdating, fetchVersions, updateCli } = useCliVersions();
  const { health: cliHealth, checking: cliHealthChecking, checkHealth } = useCliHealth();
  const { projects, sessions, loadSessions, loadProjects, loadSessionDetail, loadMoreRuns, deleteSession } = useHistory();
  const { skills, loading: skillsLoading, createSkill, updateSkill, deleteSkill } = useSkills();
  const hasLiveSessions = useLiveSessionStatus();
  const { status: updateStatus, updateVersion } = useUpdateCheck(hasLiveSessions);
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);

  const {
    activeView,
    setActiveView,
    activeSessionId,
    sessionDetail,
    sessionDetailLoading,
    sessionLoadingMore,
    handleRun,
    handleContinueRun,
    handleMissingAgentSetup,
    handleNewSession,
    handleBackFromChat,
    handleSelectSession,
    handleLoadMoreRuns,
    handleSelectProject,
    handleArchivedSession,
  } = useAppViewState({
    workspace,
    run,
    sessions,
    loadProjects,
    loadSessions,
    loadSessionDetail,
    loadMoreRuns,
    openWorkspace,
    startPipeline,
    resetRun,
    notifyError: toast.error,
  });

  useEffect(() => {
    if (!settings) return;
    checkHealth(settings);
  }, [settings, checkHealth]);

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
          onNewSession={handleNewSession}
          activeView={activeView}
          onNavigate={setActiveView}
          projects={projects}
          activeProjectPath={workspace?.path}
          onSelectProject={handleSelectProject}
          onAddProject={selectFolder}
          sessions={sessions}
          activeSessionId={activeSessionId}
          onSelectSession={handleSelectSession}
          runningSessionId={run && isActive(run.status) ? run.sessionId : undefined}
          onArchiveSession={(sessionId) => {
            void deleteSession(sessionId).then(
              () => toast.success("Session deleted."),
              () => toast.error("Failed to delete session."),
            );
            handleArchivedSession(sessionId);
          }}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          <AppContentRouter
            activeView={activeView}
            activeSessionId={activeSessionId}
            run={run}
            stageLogs={stageLogs}
            artifacts={artifacts}
            workspace={workspace}
            projects={projects}
            sessionDetail={sessionDetail}
            sessionDetailLoading={sessionDetailLoading}
            sessionLoadingMore={sessionLoadingMore}
            versions={versions}
            versionsLoading={versionsLoading}
            versionsUpdating={versionsUpdating}
            cliHealth={cliHealth}
            cliHealthChecking={cliHealthChecking}
            settings={settings}
            skills={skills}
            skillsLoading={skillsLoading}
            onSaveSettings={saveSettings}
            onFetchVersions={fetchVersions}
            onUpdateCli={updateCli}
            onCreateSkill={createSkill}
            onUpdateSkill={updateSkill}
            onDeleteSkill={deleteSkill}
            onMissingAgentSetup={handleMissingAgentSetup}
            onPausePipeline={pausePipeline}
            onResumePipeline={resumePipeline}
            onCancelPipeline={cancelPipeline}
            onRun={handleRun}
            onContinueRun={handleContinueRun}
            onNewSession={handleNewSession}
            onBackFromChat={handleBackFromChat}
            onLoadMoreRuns={handleLoadMoreRuns}
            onSelectProject={handleSelectProject}
            onAddProject={selectFolder}
          />
        </div>
        {pendingQuestion && (
          <QuestionDialog
            question={pendingQuestion}
            onAnswer={answerQuestion}
          />
        )}
        {updateStatus !== "idle" && (
          <UpdateInstallBanner
            mode={updateStatus}
            version={updateVersion}
          />
        )}
      </div>
      {openingWorkspace && <ProjectLoadingOverlay />}
    </div>
  );
}
export default App;
