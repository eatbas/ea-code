import type { ReactNode } from "react";
import { useState, useEffect, useCallback } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliVersions } from "./hooks/useCliVersions";
import { useHistory } from "./hooks/useHistory";
import { useSkills } from "./hooks/useSkills";
import { Sidebar } from "./components/Sidebar";
import type { ActiveView } from "./components/Sidebar";
import { IdleView } from "./components/IdleView";
import { ChatView } from "./components/ChatView";
import { AgentsView } from "./components/AgentsView";
import { CliSetupView } from "./components/CliSetupView";
import { SkillsView } from "./components/SkillsView";
import { QuestionDialog } from "./components/QuestionDialog";
import type { PipelineRequest } from "./types";

function App(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { workspace, selectFolder } = useWorkspace();
  const { run, logs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { versions, loading: versionsLoading, updating: versionsUpdating, error: versionsError, fetchVersions, updateCli } = useCliVersions();
  const { sessions, loadSessions, loadProjects } = useHistory();
  const { skills, loading: skillsLoading, error: skillsError, createSkill, updateSkill, deleteSkill } = useSkills();

  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [activeView, setActiveView] = useState<ActiveView>("home");
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();

  const isRunning = run && run.status !== "idle";

  // Load projects on mount
  useEffect(() => { loadProjects(); }, [loadProjects]);

  // Load sessions when workspace changes
  useEffect(() => {
    if (workspace) {
      loadSessions(workspace.path);
    }
  }, [workspace, loadSessions]);

  // Reload sessions when a pipeline run completes
  useEffect(() => {
    if (run && (run.status === "completed" || run.status === "failed" || run.status === "cancelled") && workspace) {
      loadSessions(workspace.path);
    }
  }, [run?.status, workspace, loadSessions]);

  function handleRun(prompt: string): void {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt,
      workspacePath: workspace.path,
      sessionId: activeSessionId,
    };
    startPipeline(request);
  }

  function handleNewSession(): void {
    resetRun();
    setActiveSessionId(undefined);
    setActiveView("home");
  }

  const handleSelectSession = useCallback((_sessionId: string) => {
    setActiveSessionId(_sessionId);
    setActiveView("home");
  }, []);

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
      return <AgentsView settings={settings} onSave={saveSettings} />;
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

    return (
      <IdleView
        workspace={workspace}
        onSelectFolder={selectFolder}
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
