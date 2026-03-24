import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import type { AppSettings, ProviderInfo } from "../../types";
import { sanitiseAgentAssignmentsForEnabledModels } from "../../utils/agentSettings";
import { InlineStageSlot } from "./InlineStageSlot";
import { StageCard } from "./StageCard";
import { useGroupedExtraSlots } from "./useExtraSlots";

/** Props for the AgentsView component. */
export interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  providers: ProviderInfo[];
  providersLoading?: boolean;
}

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({
  settings,
  onSave,
  providers,
  providersLoading,
}: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const draftRef = useRef<AppSettings>(settings);

  useEffect(() => {
    const sanitised = sanitiseAgentAssignmentsForEnabledModels(settings);
    draftRef.current = sanitised;
    setDraft(sanitised);
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    // Force unlimited slots by setting max high.
    const next = { ...draftRef.current, ...patch, maxPlanners: 99, maxReviewers: 99 };
    draftRef.current = next;
    setDraft(next);
    onSave(next);
  }

  const extraSlots = useGroupedExtraSlots(settings, draftRef, update);

  function handleFreshStart(): void {
    const cleared: AppSettings = {
      ...draftRef.current,
      promptEnhancerAgent: null,
      skillSelectorAgent: null,
      plannerAgent: null,
      planAuditorAgent: null,
      coderAgent: null,
      codeReviewerAgent: null,
      reviewMergerAgent: null,
      codeFixerAgent: null,
      finalJudgeAgent: null,
      executiveSummaryAgent: null,
      promptEnhancerModel: "",
      skillSelectorModel: null,
      plannerModel: null,
      planAuditorModel: null,
      coderModel: "",
      codeReviewerModel: "",
      reviewMergerModel: null,
      codeFixerModel: "",
      finalJudgeModel: "",
      executiveSummaryModel: "",
      extraPlanners: [],
      extraReviewers: [],
    };
    draftRef.current = cleared;
    setDraft(cleared);
    onSave(cleared);
  }

  const providerList = providers;
  const loading = Boolean(providersLoading);

  const cardProps = { draft, providers: providerList, providersLoading: loading, onUpdate: update };

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>

          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={handleFreshStart}
              className="rounded border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed]"
            >
              Fresh Start: Clear All
            </button>
          </div>

          {/* Prompt Enhancer + Skill Selector */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <StageCard
              label="Prompt Enhancer"
              tag="(required)"
              backendKey="promptEnhancerAgent"
              modelKey="promptEnhancerModel"
              optional={false}
              {...cardProps}
            />
            <StageCard
              label="Skill Selector"
              tag="(optional)"
              backendKey="skillSelectorAgent"
              modelKey="skillSelectorModel"
              optional={true}
              {...cardProps}
            />
          </div>

          {/* Planning & Review */}
          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium uppercase tracking-wider text-[#6b6b82]">
              Planning & Review
            </span>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              {/* Planner / Reviewer group (with inline extras) */}
              <StageCard
                label="Planner / Reviewer 1"
                tag="(required)"
                subtitle="Plans and reviews code"
                backendKey="plannerAgent"
                modelKey="plannerModel"
                groupedKeys={[
                  { backendKey: "codeReviewerAgent", modelKey: "codeReviewerModel" },
                ]}
                optional={false}
                {...cardProps}
              >
                {Array.from({ length: extraSlots.openCount }, (_, i) => (
                  <InlineStageSlot
                    key={`pr-${i + 2}`}
                    label={`Planner / Reviewer ${i + 2}`}
                    backend={draft.extraPlanners[i]?.agent ?? null}
                    model={draft.extraPlanners[i]?.model ?? null}
                    onChange={(b, m) => extraSlots.updateSlot(i, b, m)}
                    onRemove={() => extraSlots.removeSlot(i)}
                    {...cardProps}
                  />
                ))}
              </StageCard>

              {/* Auditor / Merger group */}
              <StageCard
                label="Auditor / Merger"
                tag="(required)"
                subtitle="Audits plans, merges reviews"
                backendKey="planAuditorAgent"
                modelKey="planAuditorModel"
                groupedKeys={[
                  { backendKey: "reviewMergerAgent", modelKey: "reviewMergerModel" },
                ]}
                optional={false}
                {...cardProps}
              />
            </div>
            <button
              type="button"
              onClick={extraSlots.addSlot}
              className="self-start rounded border border-dashed border-[#2e2e48] px-3 py-1.5 text-xs text-[#6b6b82] hover:border-[#6366f1] hover:text-[#6366f1]"
            >
              + Add Planner / Reviewer
            </button>
          </div>

          {/* Coding */}
          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium uppercase tracking-wider text-[#6b6b82]">
              Coding
            </span>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <StageCard
                label="Coder / Fixer"
                tag="(required)"
                subtitle="Writes and fixes code"
                backendKey="coderAgent"
                modelKey="coderModel"
                groupedKeys={[
                  { backendKey: "codeFixerAgent", modelKey: "codeFixerModel" },
                ]}
                optional={false}
                {...cardProps}
              />
            </div>
          </div>

          {/* Judge + Executive Summary */}
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <StageCard
              label="Judge"
              tag="(required)"
              backendKey="finalJudgeAgent"
              modelKey="finalJudgeModel"
              optional={false}
              {...cardProps}
            />
            <StageCard
              label="Executive Summary"
              tag="(required)"
              backendKey="executiveSummaryAgent"
              modelKey="executiveSummaryModel"
              optional={false}
              {...cardProps}
            />
          </div>

          {/* Pipeline config */}
          <div className="flex flex-col gap-3 border-t border-[#2e2e48] pt-4">
            <span className="text-sm font-medium text-[#e4e4ed]">Pipeline</span>
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-3">
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
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
