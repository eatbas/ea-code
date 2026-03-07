import type { ReactNode } from "react";
import { useState } from "react";
import { useSettings } from "./hooks/useSettings";
import { useWorkspace } from "./hooks/useWorkspace";
import { usePipeline } from "./hooks/usePipeline";
import { useCliHealth } from "./hooks/useCliHealth";
import { Header } from "./components/Header";
import { PromptPanel } from "./components/PromptPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { RunTimeline } from "./components/RunTimeline";
import { LogsPanel } from "./components/LogsPanel";
import { ArtifactsPanel } from "./components/ArtifactsPanel";
import { StatusBar } from "./components/StatusBar";
import type { PipelineRequest } from "./types";

function App(): ReactNode {
  const { settings, loading, saveSettings } = useSettings();
  const { workspace, selectFolder } = useWorkspace();
  const { run, logs, artifacts, startPipeline, cancelPipeline } = usePipeline();
  const { health, checkHealth } = useCliHealth();

  const [showSettings, setShowSettings] = useState<boolean>(false);

  const isRunning = run?.status === "running";
  const currentIteration = run?.currentIteration ?? 0;
  const maxIterations = settings?.maxIterations ?? 3;

  // Current iteration's stages for the timeline
  const currentStages =
    run && run.iterations.length > 0
      ? run.iterations[run.iterations.length - 1].stages
      : [];

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
      {/* Top header */}
      <Header
        workspace={workspace}
        showSettings={showSettings}
        onToggleSettings={() => setShowSettings((prev) => !prev)}
      />

      {/* Main content area */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left sidebar */}
        <aside className="w-80 shrink-0 border-r border-[#2e2e48] bg-[#1a1a24] overflow-y-auto">
          {showSettings && settings ? (
            <SettingsPanel
              settings={settings}
              onSave={saveSettings}
              health={health ?? undefined}
              onCheckHealth={handleCheckHealth}
            />
          ) : (
            <PromptPanel
              onRun={handleRun}
              onCancel={cancelPipeline}
              onSelectFolder={selectFolder}
              workspace={workspace}
              isRunning={isRunning}
              currentIteration={currentIteration}
              maxIterations={maxIterations}
            />
          )}
        </aside>

        {/* Right content */}
        <main className="flex flex-1 flex-col overflow-hidden">
          {/* Timeline */}
          <RunTimeline
            stages={currentStages}
            currentStage={run?.currentStage}
            iteration={currentIteration || 1}
          />

          {/* Logs + Artefacts split */}
          <div className="flex flex-1 overflow-hidden">
            <div className="flex-1 border-r border-[#2e2e48] overflow-hidden flex flex-col">
              <LogsPanel logs={logs} />
            </div>
            <div className="flex-1 overflow-hidden flex flex-col">
              <ArtifactsPanel artifacts={artifacts} />
            </div>
          </div>
        </main>
      </div>

      {/* Bottom status bar */}
      <StatusBar
        status={run?.status ?? "idle"}
        currentStage={run?.currentStage}
        iteration={run?.currentIteration}
        maxIterations={maxIterations}
        startedAt={run?.startedAt}
      />
    </div>
  );
}

export default App;
