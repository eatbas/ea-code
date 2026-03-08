import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, AgentBackend, CliHealth } from "../../types";
import { CascadingSelect } from "./CascadingSelect";

/** Configuration for a single pipeline stage row. */
interface StageConfig {
  label: string;
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
  optional: boolean;
}

/** Ordered list of pipeline stages for the agents grid. */
const STAGES: StageConfig[] = [
  { label: "Prompt Enhancer", backendKey: "promptEnhancerAgent", modelKey: "promptEnhancerModel", optional: false },
  { label: "Skill Selector", backendKey: "skillSelectorAgent", modelKey: "skillSelectorModel", optional: true },
  { label: "Planner", backendKey: "plannerAgent", modelKey: "plannerModel", optional: true },
  { label: "Plan Auditor", backendKey: "planAuditorAgent", modelKey: "planAuditorModel", optional: true },
  { label: "Coder", backendKey: "generatorAgent", modelKey: "generatorModel", optional: false },
  { label: "Code Reviewer", backendKey: "reviewerAgent", modelKey: "reviewerModel", optional: false },
  { label: "Code Fixer", backendKey: "fixerAgent", modelKey: "fixerModel", optional: false },
  { label: "Judge", backendKey: "finalJudgeAgent", modelKey: "finalJudgeModel", optional: false },
  { label: "Executive Summary", backendKey: "executiveSummaryAgent", modelKey: "executiveSummaryModel", optional: false },
];

/** Props for the AgentsView component. */
export interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  projectScoped?: boolean;
  onResetProjectSettings?: () => Promise<void>;
  cliHealth?: CliHealth | null;
  cliHealthChecking?: boolean;
}

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({
  settings,
  onSave,
  projectScoped,
  onResetProjectSettings,
  cliHealth,
  cliHealthChecking,
}: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    setDraft((prev) => ({ ...prev, ...patch }));
  }

  function handleSave(): void {
    onSave(draft);
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>
          {projectScoped && (
            <div className="rounded border border-[#2e2e48] bg-[#1a1a2e] px-3 py-2 text-xs text-[#9898b0]">
              Project override mode: settings are saved for the active workspace.
              {onResetProjectSettings && (
                <button
                  onClick={() => void onResetProjectSettings()}
                  className="ml-3 rounded border border-[#2e2e48] bg-[#24243a] px-2 py-1 text-xs text-[#e4e4ed] hover:bg-[#2e2e48]"
                >
                  Reset Project Overrides
                </button>
              )}
            </div>
          )}

          {/* Agent cards — 2-column grid */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            {STAGES.map((stage) => {
              const currentBackend = draft[stage.backendKey] as AgentBackend | null;
              const currentModel = draft[stage.modelKey] as string | null;

              return (
                <div
                  key={stage.label}
                  className="rounded-lg border border-[#2e2e48] bg-[#1a1a2e] p-4 flex flex-col gap-2"
                >
                  <span className="text-xs font-medium text-[#9898b0]">
                    {stage.label}
                    {stage.optional && (
                      <span className="ml-1 text-[#6b6b80]">(optional)</span>
                    )}
                  </span>
                  <CascadingSelect
                    backend={currentBackend as AgentBackend}
                    model={currentModel ?? ""}
                    settings={draft}
                    optional={stage.optional}
                    cliHealth={cliHealth ?? null}
                    cliHealthChecking={Boolean(cliHealthChecking)}
                    onChange={(newBackend, newModel) => {
                      update({
                        [stage.backendKey]: newBackend,
                        [stage.modelKey]: newModel,
                      });
                    }}
                  />
                </div>
              );
            })}
          </div>

          {/* Pipeline parameters */}
          <div className="flex flex-col gap-3 border-t border-[#2e2e48] pt-4">
            <span className="text-sm font-medium text-[#e4e4ed]">Pipeline</span>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Skill Selection Mode</span>
              <select
                value={draft.skillSelectionMode}
                onChange={(e) => update({ skillSelectionMode: e.target.value as AppSettings["skillSelectionMode"] })}
                className="w-44 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              >
                <option value="disable">Disable</option>
                <option value="auto">Auto (agent selects)</option>
              </select>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Max Iterations</span>
              <input
                type="number"
                min={1}
                max={10}
                value={draft.maxIterations}
                onChange={(e) => update({ maxIterations: Math.max(1, Math.min(10, Number(e.target.value))) })}
                className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={draft.requireGit}
                onChange={(e) => update({ requireGit: e.target.checked })}
                className="rounded border-[#2e2e48] accent-[#6366f1]"
              />
              <span className="text-xs text-[#9898b0]">Require Git repository</span>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Agent Max Turns</span>
              <input
                type="number"
                min={1}
                max={100}
                value={draft.agentMaxTurns}
                onChange={(e) => update({ agentMaxTurns: Math.max(1, Math.min(100, Number(e.target.value))) })}
                className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Mode</span>
              <select
                value={draft.mode}
                onChange={(e) => update({ mode: e.target.value as AppSettings["mode"] })}
                className="w-44 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              >
                <option value="workspace-write">workspace-write</option>
                <option value="diff-first">diff-first</option>
              </select>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={draft.updateCliOnRun}
                onChange={(e) => update({ updateCliOnRun: e.target.checked })}
                className="rounded border-[#2e2e48] accent-[#6366f1]"
              />
              <span className="text-xs text-[#9898b0]">Update CLIs on run start</span>
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={draft.failOnCliUpdateError}
                onChange={(e) => update({ failOnCliUpdateError: e.target.checked })}
                className="rounded border-[#2e2e48] accent-[#6366f1]"
              />
              <span className="text-xs text-[#9898b0]">Fail run on CLI update error</span>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">CLI Update Timeout (ms)</span>
              <input
                type="number"
                min={1000}
                step={1000}
                value={draft.cliUpdateTimeoutMs}
                onChange={(e) => update({ cliUpdateTimeoutMs: Math.max(1000, Number(e.target.value)) })}
                className="w-36 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
            </label>
          </div>

          {/* Save */}
          <button
            onClick={handleSave}
            className="self-start rounded bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] hover:bg-white transition-colors"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
