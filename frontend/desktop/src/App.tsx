import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
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
import { QuestionDialog } from "./components/QuestionDialog";
import { UpdateInstallBanner } from "./components/shared/UpdateInstallBanner";
import type { PipelineRequest, PipelineRun, RunOptions, SessionDetail } from "./types";

/** Whether the pipeline is actively in progress (running or awaiting user input). */
function isRunActive(run: PipelineRun | null): boolean {
  return !!run && (run.status === "running" || run.status === "waiting_for_input");
}

/** Whether the pipeline has reached a terminal state (completed, failed, or cancelled). */
function isRunTerminal(run: PipelineRun | null): boolean {
  return !!run && (run.status === "completed" || run.status === "failed" || run.status === "cancelled");
}

function App(): ReactNode {
  const { workspace, openWorkspace, selectFolder } = useWorkspace();
  const { settings, loading, saveSettings } = useSettings();
  const { run, stageLogs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { versions, loading: versionsLoading, updating: versionsUpdating, error: versionsError, fetchVersions, updateCli } = useCliVersions();
  const { health: cliHealth, checking: cliHealthChecking, checkHealth } = useCliHealth();
  const { projects, sessions, loadSessions, loadProjects, loadSessionDetail, deleteSession } = useHistory();
  const { skills, loading: skillsLoading, error: skillsError, createSkill, updateSkill, deleteSkill } = useSkills();
  const { installing: installingUpdate, updateVersion } = useUpdateCheck();

  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [activeView, setActiveView] = useState<ActiveView>("home");
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [sessionDetail, setSessionDetail] = useState<SessionDetail | null>(null);
  const [sessionDetailLoading, setSessionDetailLoading] = useState(false);

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

  // Reload sessions when a pipeline run starts or completes
  useEffect(() => {
    if ((isRunActive(run) || isRunTerminal(run)) && workspace) {
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

  function handleMissingAgentSetup(): void {
    setActiveView("agents");
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
    // Actively running pipeline always takes priority (blocks navigation)
    const pipelineActive = isRunActive(run);
    // Terminal pipeline shown only when on home view
    const pipelineTerminal = isRunTerminal(run);
    const showChat = pipelineActive || (pipelineTerminal && activeView === "home" && !activeSessionId);

    if (run && showChat) {
      return (
        <ChatView
          run={run}
          stageLogs={stageLogs}
          artifacts={artifacts}
          cliHealth={cliHealth}
          settings={settings}
          onMissingAgentSetup={handleMissingAgentSetup}
          onCancel={cancelPipeline}
          onBackToHome={handleNewSession}
          onContinue={(options) => {
            if (run.sessionId) setActiveSessionId(run.sessionId);
            handleRun(options);
          }}
          onViewSession={() => {
            if (run.sessionId) {
              void handleSelectSession(run.sessionId);
              resetRun();
            }
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
          settings={settings}
          onMissingAgentSetup={handleMissingAgentSetup}
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
        isRunning={isRunActive(run)}
        hasTerminalRun={isRunTerminal(run)}
        onGoToRun={() => {
          setActiveSessionId(undefined);
          setSessionDetail(null);
          setActiveView("home");
        }}
        onArchiveSession={(sessionId) => {
          void deleteSession(sessionId);
          if (activeSessionId === sessionId) {
            setActiveSessionId(undefined);
            setSessionDetail(null);
          }
        }}
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

      {installingUpdate && <UpdateInstallBanner version={updateVersion} />}
    </div>
  );
}

export default App;
