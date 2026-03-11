import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { useToast } from "./components/shared/Toast";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliVersions } from "./hooks/useCliVersions";
import { useCliHealth } from "./hooks/useCliHealth";
import { useHistory } from "./hooks/useHistory";
import { useSkills } from "./hooks/useSkills";
import { useUpdateCheck } from "./hooks/useUpdateCheck";
import { Sidebar } from "./components/Sidebar";
import type { ActiveView } from "./components/Sidebar";
import { IdleView } from "./components/IdleView";
import { ChatView } from "./components/ChatView";
import { SessionDetailView } from "./components/SessionDetailView";
import { AgentsView } from "./components/AgentsView";
import { CliSetupView } from "./components/CliSetupView";
import { SkillsView } from "./components/SkillsView";
import { McpView } from "./components/McpView";
import { AppSettingsView } from "./components/AppSettingsView";
import { QuestionDialog } from "./components/QuestionDialog";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import { ProjectLoadingOverlay } from "./components/shared/ProjectLoadingOverlay";
import type { PipelineRequest, PipelineRun, RunOptions, SessionDetail } from "./types";
function isRunActive(run: PipelineRun | null): boolean {
  return !!run && (run.status === "running" || run.status === "waiting_for_input" || run.status === "paused");
}
function isRunTerminal(run: PipelineRun | null): boolean {
  return !!run && (run.status === "completed" || run.status === "failed" || run.status === "cancelled");
}
function App(): ReactNode {
  const toast = useToast();
  const { workspace, openingWorkspace, openWorkspace, selectFolder } = useWorkspace();
  const { settings, loading, saveSettings } = useSettings();
  const { run, stageLogs, artifacts, pendingQuestion, startPipeline, pausePipeline, resumePipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { versions, loading: versionsLoading, updating: versionsUpdating, fetchVersions, updateCli } = useCliVersions();
  const { health: cliHealth, checking: cliHealthChecking, checkHealth } = useCliHealth();
  const { projects, sessions, loadSessions, loadProjects, loadSessionDetail, loadMoreRuns, deleteSession } = useHistory();
  const { skills, loading: skillsLoading, createSkill, updateSkill, deleteSkill } = useSkills();
  const { installing: installingUpdate, updateVersion } = useUpdateCheck();
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [activeView, setActiveView] = useState<ActiveView>("home");
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [sessionDetail, setSessionDetail] = useState<SessionDetail | null>(null);
  const [sessionDetailLoading, setSessionDetailLoading] = useState(false);
  const [sessionLoadingMore, setSessionLoadingMore] = useState(false);
  useEffect(() => { loadProjects(); }, [loadProjects]);
  useEffect(() => {
    if (workspace) {
      loadProjects();
      loadSessions(workspace.path);
      setActiveSessionId(undefined);
      setSessionDetail(null);
    }
  }, [workspace, loadProjects, loadSessions]);
  useEffect(() => {
    if ((isRunActive(run) || isRunTerminal(run)) && workspace) {
      loadSessions(workspace.path);
    }
  }, [run?.status, workspace, loadSessions]);
  const hasRunningSession = sessions.some((s) => s.lastStatus === "running" || s.lastStatus === "waiting_for_input" || s.lastStatus === "paused");
  useEffect(() => {
    if (!hasRunningSession || !workspace) return;
    const interval = setInterval(async () => {
      try {
        loadSessions(workspace.path);
        if (activeSessionId) {
          const detail = await loadSessionDetail(activeSessionId);
          setSessionDetail(detail);
        }
      } catch { /* silent - will retry next interval */ }
    }, 3000);
    return () => clearInterval(interval);
  }, [hasRunningSession, workspace, activeSessionId, loadSessions, loadSessionDetail]);
  useEffect(() => {
    if (!settings) return;
    void checkHealth(settings);
  }, [settings, checkHealth]);
  async function handleRun(options: RunOptions, sessionIdOverride?: string): Promise<void> {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt: options.prompt,
      workspacePath: workspace.path,
      sessionId: sessionIdOverride ?? activeSessionId,
      directTask: options.directTask || undefined,
      directTaskAgent: options.directTaskAgent,
      directTaskModel: options.directTaskModel,
      noPlan: options.noPlan || undefined,
    };
    startPipeline(request);
  }
  function handleMissingAgentSetup(): void {
    setActiveView("agents");
  }
  function handleNewSession(): void {
    resetRun();
    setActiveSessionId(undefined);
    setSessionDetail(null);
    setActiveView("home");
    if (workspace) loadSessions(workspace.path);
  }
  const handleSelectSession = useCallback(async (_sessionId: string): Promise<void> => {
    const isCurrentRun = run && (isRunActive(run) || isRunTerminal(run)) && run.sessionId === _sessionId;
    if (isCurrentRun) {
      setActiveSessionId(undefined);
      setSessionDetail(null);
      setActiveView("home");
      return;
    }
    setActiveSessionId(_sessionId);
    setActiveView("home");
    setSessionDetailLoading(true);
    try {
      const detail = await loadSessionDetail(_sessionId);
      setSessionDetail(detail);
    } catch {
      toast.error("Failed to load session.");
      setSessionDetail(null);
    } finally {
      setSessionDetailLoading(false);
    }
  }, [loadSessionDetail, toast, run]);
  const handleLoadMoreRuns = useCallback(async (): Promise<void> => {
    if (!activeSessionId || !sessionDetail) return;
    setSessionLoadingMore(true);
    try {
      const older = await loadMoreRuns(activeSessionId, sessionDetail.runs.length);
      setSessionDetail((prev) => {
        if (!prev) return prev;
        return { ...prev, runs: [...older.runs, ...prev.runs], totalRuns: older.totalRuns };
      });
    } catch {
      toast.error("Failed to load earlier runs.");
    } finally {
      setSessionLoadingMore(false);
    }
  }, [activeSessionId, sessionDetail, loadMoreRuns, toast]);

  const handleSelectProject = useCallback(async (projectPath: string): Promise<void> => {
    await openWorkspace(projectPath);
    setActiveView("home");
  }, [openWorkspace]);
  function renderContent(): ReactNode {
    const pipelineActive = isRunActive(run);
    const pipelineTerminal = isRunTerminal(run);
    const isHomeRootView = activeView === "home" && !activeSessionId;
    const showChat = isHomeRootView && (pipelineActive || pipelineTerminal);
    if (run && showChat) {
      return (
        <ChatView
          run={run}
          stageLogs={stageLogs}
          artifacts={artifacts}
          cliHealth={cliHealth}
          settings={settings}
          onMissingAgentSetup={handleMissingAgentSetup}
          onPause={() => { void pausePipeline(); }}
          onResume={() => { void resumePipeline(); }}
          onCancel={() => { void cancelPipeline(); }}
          onNewSession={handleNewSession}
          onContinue={(options) => {
            if (run.sessionId) setActiveSessionId(run.sessionId);
            handleRun(options, run.sessionId);
          }}
        />
      );
    }
    if (activeView === "agents" && settings) {
      return (
        <AgentsView
          settings={settings}
          onSave={saveSettings}
          cliHealth={cliHealth}
          cliHealthChecking={cliHealthChecking}
        />
      );
    }
    if (activeView === "cli-setup" && settings) {
      return (
        <CliSetupView
          settings={settings}
          versions={versions}
          loading={versionsLoading}
          updating={versionsUpdating}
          onFetchVersions={fetchVersions}
          onUpdateCli={updateCli}
          onSave={saveSettings}
        />
      );
    }
    if (activeView === "skills") {
      return (
        <SkillsView
          skills={skills}
          loading={skillsLoading}
          onCreate={createSkill}
          onUpdate={updateSkill}
          onDelete={deleteSkill}
        />
      );
    }
    if (activeView === "mcp") {
      return <McpView />;
    }
    if (activeView === "app-settings") {
      return <AppSettingsView />;
    }
    if (activeSessionId) {
      return (
        <SessionDetailView
          sessionDetail={sessionDetail}
          loading={sessionDetailLoading}
          stageLogs={stageLogs}
          activeRunId={run?.id}
          cliHealth={cliHealth}
          settings={settings}
          onMissingAgentSetup={handleMissingAgentSetup}
          onRun={handleRun}
          onPauseRun={(runId) => { void pausePipeline(runId); }}
          onResumeRun={(runId) => { void resumePipeline(runId); }}
          onCancelRun={(runId) => { void cancelPipeline(runId); }}
          onLoadMore={handleLoadMoreRuns}
          loadingMore={sessionLoadingMore}
          onBackToHome={handleNewSession}
        />
      );
    }
    return (
      <IdleView
        workspace={workspace}
        workspacePath={workspace?.path}
        projects={projects}
        cliHealth={cliHealth}
        settings={settings}
        onMissingAgentSetup={handleMissingAgentSetup}
        onSelectProject={handleSelectProject}
        onAddProject={selectFolder}
        onRun={handleRun}
      />
    );
  }
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
          runningSessionId={run && (run.status === "running" || run.status === "waiting_for_input") ? run.sessionId : undefined}
          onArchiveSession={(sessionId) => {
            void deleteSession(sessionId).then(
              () => toast.success("Session archived."),
              () => toast.error("Failed to archive session."),
            );
            if (activeSessionId === sessionId) {
              setActiveSessionId(undefined);
              setSessionDetail(null);
            }
          }}
        />
        <div className="flex min-h-0 flex-1 flex-col overflow-hidden">
          {renderContent()}
        </div>
        {pendingQuestion && (
          <QuestionDialog
            question={pendingQuestion}
            onAnswer={answerQuestion}
          />
        )}
        {installingUpdate && <UpdateInstallBanner version={updateVersion} />}
      </div>
      {openingWorkspace && <ProjectLoadingOverlay />}
    </div>
  );
}
export default App;
