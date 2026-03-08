import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliVersions } from "./hooks/useCliVersions";
import { useCliHealth } from "./hooks/useCliHealth";
import { useHistory } from "./hooks/useHistory";
import { useSkills } from "./hooks/useSkills";
import { Sidebar } from "./components/Sidebar";
import type { ActiveView } from "./components/Sidebar";
import { IdleView } from "./components/IdleView";
import { ChatView } from "./components/ChatView";
import { SessionDetailView } from "./components/SessionDetailView";
import { AgentsView } from "./components/AgentsView";
import { CliSetupView } from "./components/CliSetupView";
import { SkillsView } from "./components/SkillsView";
import { McpView } from "./components/McpView";
import { QuestionDialog } from "./components/QuestionDialog";
import type { PipelineRequest, RunOptions, SessionDetail } from "./types";

function App(): ReactNode {
  const { workspace, openWorkspace, selectFolder } = useWorkspace();
  const { settings, loading, saveSettings, clearProjectSettings } = useSettings(workspace?.path);
  const { run, logs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { versions, loading: versionsLoading, updating: versionsUpdating, error: versionsError, fetchVersions, updateCli } = useCliVersions();
  const { health: cliHealth, checking: cliHealthChecking, checkHealth } = useCliHealth();
  const { projects, sessions, loadSessions, loadProjects, loadSessionDetail } = useHistory();
  const { skills, loading: skillsLoading, error: skillsError, createSkill, updateSkill, deleteSkill } = useSkills();

  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [activeView, setActiveView] = useState<ActiveView>("home");
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [sessionDetail, setSessionDetail] = useState<SessionDetail | null>(null);
  const [sessionDetailLoading, setSessionDetailLoading] = useState(false);

  const isRunning = run && run.status !== "idle";

  // Load projects on mount
  useEffect(() => { loadProjects(); }, [loadProjects]);

  // Load sessions when workspace changes
  useEffect(() => {
    if (workspace) {
      loadProjects();
      loadSessions(workspace.path);
      setActiveSessionId(undefined);
      setSessionDetail(null);
    }
  }, [workspace, loadProjects, loadSessions]);

  // Reload sessions when a pipeline run completes
  useEffect(() => {
    if (run && (run.status === "completed" || run.status === "failed" || run.status === "cancelled") && workspace) {
      loadSessions(workspace.path);
    }
  }, [run?.status, workspace, loadSessions]);

  // Refresh CLI availability whenever persisted settings change.
  useEffect(() => {
    if (!settings) return;
    void checkHealth(settings);
  }, [settings, checkHealth]);

  function handleRun(options: RunOptions): void {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt: options.prompt,
      workspacePath: workspace.path,
      sessionId: activeSessionId,
      directTask: options.directTask || undefined,
      directTaskAgent: options.directTaskAgent,
      directTaskModel: options.directTaskModel,
      noPlan: options.noPlan || undefined,
    };
    startPipeline(request);
  }

  function handleNewSession(): void {
    resetRun();
    setActiveSessionId(undefined);
    setSessionDetail(null);
    setActiveView("home");
  }

  const handleSelectSession = useCallback(async (_sessionId: string): Promise<void> => {
    setActiveSessionId(_sessionId);
    setActiveView("home");
    setSessionDetailLoading(true);
    try {
      const detail = await loadSessionDetail(_sessionId);
      setSessionDetail(detail);
    } catch (err) {
      console.error("Failed to load session detail:", err);
      setSessionDetail(null);
    } finally {
      setSessionDetailLoading(false);
    }
  }, [loadSessionDetail]);

  const handleSelectProject = useCallback(async (projectPath: string): Promise<void> => {
    await openWorkspace(projectPath);
    setActiveView("home");
  }, [openWorkspace]);

  /** Render the main content area based on active view. */
  function renderContent(): ReactNode {
    if (isRunning) {
      return (
        <ChatView
          run={run}
          logs={logs}
          artifacts={artifacts}
          onCancel={cancelPipeline}
          onBackToHome={handleNewSession}
        />
      );
    }

    if (activeView === "agents" && settings) {
      return (
        <AgentsView
          settings={settings}
          onSave={saveSettings}
          projectScoped={Boolean(workspace?.path)}
          onResetProjectSettings={workspace?.path ? clearProjectSettings : undefined}
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
          error={versionsError}
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
          error={skillsError}
          onCreate={createSkill}
          onUpdate={updateSkill}
          onDelete={deleteSkill}
        />
      );
    }

    if (activeView === "mcp") {
      return <McpView />;
    }

    if (activeSessionId) {
      return (
        <SessionDetailView
          sessionDetail={sessionDetail}
          loading={sessionDetailLoading}
          cliHealth={cliHealth}
          onRun={handleRun}
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
    <div className="flex h-full bg-[#0f0f14]">
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
      />

      <div className="flex flex-1 flex-col overflow-hidden">
        {renderContent()}
      </div>

      {pendingQuestion && (
        <QuestionDialog
          question={pendingQuestion}
          onAnswer={answerQuestion}
        />
      )}
    </div>
  );
}

export default App;
