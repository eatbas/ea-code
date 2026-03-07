import type { ReactNode } from "react";
import { useState } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliHealth } from "./hooks/useCliHealth";
import { Sidebar } from "./components/Sidebar";
import type { ActiveView } from "./components/Sidebar";
import { IdleView } from "./components/IdleView";
import { ChatView } from "./components/ChatView";
import { AgentsView } from "./components/AgentsView";
import { CliSetupView } from "./components/CliSetupView";
import { QuestionDialog } from "./components/QuestionDialog";
import type { PipelineRequest } from "./types";

function App(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { workspace, selectFolder } = useWorkspace();
  const { run, logs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { health, checkHealth } = useCliHealth();

  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(false);
  const [activeView, setActiveView] = useState<ActiveView>("home");

  const isRunning = run && run.status !== "idle";

  function handleRun(prompt: string): void {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt,
      workspacePath: workspace.path,
    };
    startPipeline(request);
  }

  function handleNewSession(): void {
    resetRun();
    setActiveView("home");
  }

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
          onSave={saveSettings}
          health={health ?? undefined}
          onCheckHealth={() => { if (settings) checkHealth(settings); }}
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
