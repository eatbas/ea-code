import type { ReactNode } from "react";
import { useCallback, useState } from "react";
import type { AppSettings, CodePipelineSettings, PipelineAgent, ProviderInfo } from "../../types";
import { PipelineAgentRow } from "./PipelineAgentRow";
import { useToast } from "../shared/Toast";

interface CodePipelineCardProps {
  settings: AppSettings;
  providers: ProviderInfo[];
  onSave: (settings: AppSettings) => void;
}

function defaultAgent(providers: ProviderInfo[]): PipelineAgent {
  const first = providers[0];
  return { provider: first?.name ?? "", model: first?.models[0] ?? "" };
}

function ensurePipeline(
  settings: AppSettings,
  providers: ProviderInfo[],
): CodePipelineSettings {
  if (settings.codePipeline) return settings.codePipeline;
  const agent = defaultAgent(providers);
  return {
    planners: [{ ...agent }],
    coder: { ...agent },
    reviewers: [{ ...agent }],
  };
}

export function CodePipelineCard({
  settings,
  providers,
  onSave,
}: CodePipelineCardProps): ReactNode {
  const toast = useToast();
  const [openSelect, setOpenSelect] = useState<string | null>(null);
  const pipeline = ensurePipeline(settings, providers);

  const save = useCallback(
    (next: CodePipelineSettings) => {
      onSave({ ...settings, codePipeline: next });
      toast.success("Code pipeline updated.");
    },
    [settings, onSave, toast],
  );

  /** Reviewers always mirror planners — keep them in sync. */
  function savePlanners(planners: PipelineAgent[]): void {
    save({ ...pipeline, planners, reviewers: planners });
  }

  function updatePlanner(index: number, agent: PipelineAgent): void {
    savePlanners(pipeline.planners.map((p, i) => (i === index ? agent : p)));
  }

  function addPlanner(): void {
    const usedProviders = new Set(pipeline.planners.map((p) => p.provider));
    const unused = providers.find((p) => !usedProviders.has(p.name));
    if (!unused) return;
    savePlanners([...pipeline.planners, { provider: unused.name, model: unused.models[0] ?? "" }]);
  }

  function removePlanner(index: number): void {
    if (pipeline.planners.length <= 1) return;
    savePlanners(pipeline.planners.filter((_, i) => i !== index));
  }

  if (providers.length === 0) {
    return null;
  }

  return (
    <div className="rounded-lg border border-edge bg-panel p-5">
      <div className="flex items-center gap-2">
        <span className="inline-flex items-center rounded-lg border border-edge bg-elevated px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-fg">
          Code Pipeline
        </span>
      </div>
      <p className="mt-3 text-xs text-fg-muted">
        Plan, code, review, and fix in a multi-step pipeline.
        Planners also act as reviewers.
      </p>

      {/* Planners / Reviewers + Coder */}
      <StageSection
        label="Planners / Reviewers"
        agents={pipeline.planners}
        providers={providers}
        stagePrefix="planner"
        openSelect={openSelect}
        onOpenSelectChange={setOpenSelect}
        onUpdate={updatePlanner}
        onRemove={removePlanner}
        onAdd={addPlanner}
        addLabel="Add Planner / Reviewer"
        firstHint="merges plans"
        uniqueProviders
        coderAgent={pipeline.coder}
        coderProviders={providers}
        onCoderChange={(agent) => save({ ...pipeline, coder: agent })}
      />
    </div>
  );
}

interface StageSectionProps {
  label: string;
  agents: PipelineAgent[];
  providers: ProviderInfo[];
  stagePrefix: string;
  openSelect: string | null;
  onOpenSelectChange: (key: string | null) => void;
  onUpdate: (index: number, agent: PipelineAgent) => void;
  onRemove: (index: number) => void;
  onAdd: () => void;
  addLabel: string;
  /** Hint shown next to the first agent row. */
  firstHint?: string;
  /** Enforce unique providers across agent rows. */
  uniqueProviders?: boolean;
  /** Coder agent rendered on the first row's right side. */
  coderAgent?: PipelineAgent;
  coderProviders?: ProviderInfo[];
  onCoderChange?: (agent: PipelineAgent) => void;
}

function StageSection({
  label,
  agents,
  providers,
  stagePrefix,
  openSelect,
  onOpenSelectChange,
  onUpdate,
  onRemove,
  onAdd,
  addLabel,
  firstHint,
  uniqueProviders,
  coderAgent,
  coderProviders,
  onCoderChange,
}: StageSectionProps): ReactNode {
  const usedProviders = uniqueProviders
    ? new Set(agents.map((a) => a.provider))
    : undefined;

  return (
    <div className="mt-6 flex items-start gap-14">
      {/* Planners / Reviewers column */}
      <div>
        <p className="mb-4 text-xs font-semibold uppercase tracking-wider text-fg">
          {label}
        </p>
        <div className="flex flex-col gap-2">
          {agents.map((agent, index) => (
            <div key={`${stagePrefix}-${String(index)}`} className="flex items-center gap-2">
              <PipelineAgentRow
                agent={agent}
                providers={providers}
                slotKey={`${stagePrefix}-${String(index)}`}
                openSelect={openSelect}
                onOpenSelectChange={onOpenSelectChange}
                onChange={(next) => onUpdate(index, next)}
                removable={agents.length > 1}
                onRemove={() => onRemove(index)}
                excludeProviders={usedProviders}
              />
              {index === 0 && firstHint && (
                <span className="whitespace-nowrap text-[10px] italic text-fg-faint">
                  {firstHint}
                </span>
              )}
            </div>
          ))}
        </div>
        <button
          type="button"
          onClick={onAdd}
          disabled={uniqueProviders && agents.length >= providers.length}
          className="mt-2 flex items-center gap-1.5 rounded-lg px-2 py-1.5 text-xs text-fg-muted transition-colors hover:bg-elevated hover:text-fg disabled:cursor-not-allowed disabled:opacity-40"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          {addLabel}
        </button>
      </div>

      {/* Coder column */}
      {coderAgent && coderProviders && onCoderChange && (
        <div>
          <p className="mb-4 text-xs font-semibold uppercase tracking-wider text-fg">
            Coder
          </p>
          <PipelineAgentRow
            agent={coderAgent}
            providers={coderProviders}
            slotKey="coder"
            openSelect={openSelect}
            onOpenSelectChange={onOpenSelectChange}
            onChange={onCoderChange}
          />
        </div>
      )}
    </div>
  );
}
