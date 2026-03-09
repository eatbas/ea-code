import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, AgentBackend, CliHealth } from "../../types";
import { sanitiseAgentAssignmentsForEnabledModels } from "../../utils/agentSettings";
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
  cliHealth?: CliHealth | null;
  cliHealthChecking?: boolean;
}

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({
  settings,
  onSave,
  cliHealth,
  cliHealthChecking,
}: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);

  useEffect(() => {
    setDraft(sanitiseAgentAssignmentsForEnabledModels(settings));
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    setDraft((prev) => ({ ...prev, ...patch }));
  }

  function handleSave(): void {
    onSave(draft);
  }

  function handleFreshStart(): void {
    const cleared: AppSettings = {
      ...draft,
      promptEnhancerAgent: null,
      skillSelectorAgent: null,
      plannerAgent: null,
      planAuditorAgent: null,
      generatorAgent: null,
      reviewerAgent: null,
      fixerAgent: null,
      finalJudgeAgent: null,
      executiveSummaryAgent: null,
      promptEnhancerModel: "",
      skillSelectorModel: null,
      plannerModel: null,
      planAuditorModel: null,
      generatorModel: "",
      reviewerModel: "",
      fixerModel: "",
      finalJudgeModel: "",
      executiveSummaryModel: "",
    };
    setDraft(cleared);
    onSave(cleared);
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>
          <p className="text-xs text-[#6b6b82]">
            Roles marked as minimum must be set before prompts can be sent.
          </p>
          <div className="flex items-center gap-2">
            <button
              onClick={handleFreshStart}
              className="rounded border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed]"
            >
              Fresh Start: Clear All Agent Selections
            </button>
          </div>

          {/* Agent cards — 2-column grid */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            {STAGES.map((stage) => {
              const currentBackend = draft[stage.backendKey] as AgentBackend | null;
              const currentModel = draft[stage.modelKey] as string | null;

              return (
                <div
                  key={stage.label}
                  className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4 flex flex-col gap-2"
                >
                  <span className="text-xs font-medium text-[#9898b0]">
                    {stage.label}
                    <span className="ml-1 text-[#6b6b80]">
                      {stage.optional ? "(optional)" : "(minimum)"}
                    </span>
                  </span>
                  <CascadingSelect
                    backend={currentBackend}
                    model={currentModel}
                    settings={draft}
                    optional={stage.optional}
                    cliHealth={cliHealth ?? null}
                    cliHealthChecking={Boolean(cliHealthChecking)}
                    onChange={(newBackend, newModel) => {
                      update({
                        [stage.backendKey]: newBackend,
                        [stage.modelKey]: newModel ?? "",
                      } as Partial<AppSettings>);
                    }}
                  />
                </div>
              );
            })}
          </div>

          {/* Pipeline parameters */}
          <div className="flex flex-col gap-3 border-t border-[#2e2e48] pt-4">
            <span className="text-sm font-medium text-[#e4e4ed]">Pipeline</span>
            <div className="grid grid-cols-2 gap-4">
              <label className="flex flex-col gap-1">
                <span className="text-xs font-medium text-[#9898b0]">Max Iterations</span>
                <input
                  type="number"
                  min={1}
                  max={10}
                  value={draft.maxIterations}
                  onChange={(e) => update({ maxIterations: Math.max(1, Math.min(10, Number(e.target.value))) })}
                  className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#3e3e58] focus:outline-none"
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs font-medium text-[#9898b0]">Agent Max Turns</span>
                <input
                  type="number"
                  min={1}
                  max={100}
                  value={draft.agentMaxTurns}
                  onChange={(e) => update({ agentMaxTurns: Math.max(1, Math.min(100, Number(e.target.value))) })}
                  className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#3e3e58] focus:outline-none"
                />
              </label>
            </div>
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
