import type { ReactNode } from "react";
import { useState } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliHealth } from "./hooks/useCliHealth";
import { IdleView } from "./components/IdleView";
import { ChatView } from "./components/ChatView";
import { SettingsPanel } from "./components/SettingsPanel";
import { QuestionDialog } from "./components/QuestionDialog";
import type { PipelineRequest } from "./types";

function App(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { workspace, selectFolder } = useWorkspace();
  const { run, logs, artifacts, pendingQuestion, startPipeline, cancelPipeline, answerQuestion, resetRun } = usePipeline();
  const { health, checkHealth } = useCliHealth();

  const [showSettings, setShowSettings] = useState<boolean>(false);

  const isIdle = !run || run.status === "idle";

  function handleRun(prompt: string): void {
    if (!workspace) return;
    const request: PipelineRequest = {
      prompt,
      workspacePath: workspace.path,
    };
    startPipeline(request);
  }

  function handleCheckHealth(): void {
    if (settings) {
      checkHealth(settings);
    }
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Loading...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      {/* Settings modal overlay */}
      {showSettings && settings && (
        <SettingsPanel
          settings={settings}
          onSave={saveSettings}
          health={health ?? undefined}
          onCheckHealth={handleCheckHealth}
          onClose={() => setShowSettings(false)}
        />
      )}

      {isIdle ? (
        <IdleView
          workspace={workspace}
          onSelectFolder={selectFolder}
          onRun={handleRun}
          onOpenSettings={() => setShowSettings(true)}
        />
      ) : (
        <ChatView
          run={run}
          logs={logs}
          artifacts={artifacts}
          onCancel={cancelPipeline}
          onBackToHome={resetRun}
          onOpenSettings={() => setShowSettings(true)}
        />
      )}

      {/* Question dialog overlay */}
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
