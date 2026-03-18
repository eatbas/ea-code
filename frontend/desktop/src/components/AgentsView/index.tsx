import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import type { AppSettings, CliHealth } from "../../types";
import { sanitiseAgentAssignmentsForEnabledModels } from "../../utils/agentSettings";
import { InlineStageSlot } from "./InlineStageSlot";
import { StageCard } from "./StageCard";
import { useParallelSlotGroup } from "./useParallelSlotGroup";

/** Props for the AgentsView component. */
export interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  cliHealth?: CliHealth | null;
  cliHealthChecking?: boolean;
}

const PLANNER_SLOT_KEYS = {
  slot2AgentKey: "planner2Agent",
  slot2ModelKey: "planner2Model",
  slot3AgentKey: "planner3Agent",
  slot3ModelKey: "planner3Model",
} as const;

const REVIEWER_SLOT_KEYS = {
  slot2AgentKey: "codeReviewer2Agent",
  slot2ModelKey: "codeReviewer2Model",
  slot3AgentKey: "codeReviewer3Agent",
  slot3ModelKey: "codeReviewer3Model",
} as const;

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({
  settings,
  onSave,
  cliHealth,
  cliHealthChecking,
}: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);
  const draftRef = useRef<AppSettings>(settings);

  useEffect(() => {
    const sanitised = sanitiseAgentAssignmentsForEnabledModels(settings);
    draftRef.current = sanitised;
    setDraft(sanitised);
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    const next = { ...draftRef.current, ...patch };
    draftRef.current = next;
    setDraft(next);
    onSave(next);
  }

  const plannerSlots = useParallelSlotGroup(settings, draftRef, update, PLANNER_SLOT_KEYS);
  const reviewerSlots = useParallelSlotGroup(settings, draftRef, update, REVIEWER_SLOT_KEYS);

  function handleFreshStart(): void {
    const cleared: AppSettings = {
      ...draftRef.current,
      promptEnhancerAgent: null,
      skillSelectorAgent: null,
      plannerAgent: null,
      planner2Agent: null,
      planner3Agent: null,
      planAuditorAgent: null,
      coderAgent: null,
      codeReviewerAgent: null,
      codeReviewer2Agent: null,
      codeReviewer3Agent: null,
      reviewMergerAgent: null,
      codeFixerAgent: null,
      finalJudgeAgent: null,
      executiveSummaryAgent: null,
      promptEnhancerModel: "",
      skillSelectorModel: null,
      plannerModel: null,
      planner2Model: null,
      planner3Model: null,
      planAuditorModel: null,
      coderModel: "",
      codeReviewerModel: "",
      codeReviewer2Model: null,
      codeReviewer3Model: null,
      reviewMergerModel: null,
      codeFixerModel: "",
      finalJudgeModel: "",
      executiveSummaryModel: "",
    };
    draftRef.current = cleared;
    setDraft(cleared);
    onSave(cleared);
  }

  const health = cliHealth ?? null;
  const checking = Boolean(cliHealthChecking);
  const plannerCount = plannerSlots.activeCount;
  const reviewerCount = reviewerSlots.activeCount;

  const cardProps = { draft, cliHealth: health, cliHealthChecking: checking, onUpdate: update };

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

          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium uppercase tracking-wider text-[#6b6b82]">
              Planning
            </span>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <StageCard
                label="Planner 1"
                tag="(optional)"
                backendKey="plannerAgent"
                modelKey="plannerModel"
                optional={true}
                {...cardProps}
              >
                {plannerSlots.extraSlots.slot2 && (
                  <InlineStageSlot
                    label="Planner 2"
                    backendKey="planner2Agent"
                    modelKey="planner2Model"
                    optional={true}
                    onRemove={() => plannerSlots.removeSlot("slot2")}
                    {...cardProps}
                  />
                )}
                {plannerSlots.extraSlots.slot3 && (
                  <InlineStageSlot
                    label="Planner 3"
                    backendKey="planner3Agent"
                    modelKey="planner3Model"
                    optional={true}
                    onRemove={() => plannerSlots.removeSlot("slot3")}
                    {...cardProps}
                  />
                )}
              </StageCard>
              {plannerCount > 0 && (
                <StageCard
                  label="Plan Auditor"
                  tag={plannerCount > 1 ? "(auto — merges & audits)" : "(auto — audits plan)"}
                  backendKey="planAuditorAgent"
                  modelKey="planAuditorModel"
                  optional={true}
                  {...cardProps}
                />
              )}
            </div>
            {plannerSlots.openCount < 2 && (
              <button
                type="button"
                onClick={plannerSlots.addSlot}
                className="self-start rounded border border-dashed border-[#2e2e48] px-3 py-1.5 text-xs text-[#6b6b82] hover:border-[#6366f1] hover:text-[#6366f1]"
              >
                + Add Planner
              </button>
            )}
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <StageCard
              label="Coder"
              tag="(required)"
              backendKey="coderAgent"
              modelKey="coderModel"
              optional={false}
              {...cardProps}
            />
          </div>

          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium uppercase tracking-wider text-[#6b6b82]">
              Review
            </span>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <StageCard
                label="Reviewer 1"
                tag="(required)"
                backendKey="codeReviewerAgent"
                modelKey="codeReviewerModel"
                optional={false}
                {...cardProps}
              >
                {reviewerSlots.extraSlots.slot2 && (
                  <InlineStageSlot
                    label="Reviewer 2"
                    backendKey="codeReviewer2Agent"
                    modelKey="codeReviewer2Model"
                    optional={true}
                    onRemove={() => reviewerSlots.removeSlot("slot2")}
                    {...cardProps}
                  />
                )}
                {reviewerSlots.extraSlots.slot3 && (
                  <InlineStageSlot
                    label="Reviewer 3"
                    backendKey="codeReviewer3Agent"
                    modelKey="codeReviewer3Model"
                    optional={true}
                    onRemove={() => reviewerSlots.removeSlot("slot3")}
                    {...cardProps}
                  />
                )}
              </StageCard>
              {reviewerCount >= 2 && (
                <StageCard
                  label="Review Merger"
                  tag="(auto — combines reviews)"
                  backendKey="reviewMergerAgent"
                  modelKey="reviewMergerModel"
                  optional={true}
                  {...cardProps}
                />
              )}
            </div>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <StageCard
                label="Code Fixer"
                tag="(required)"
                backendKey="codeFixerAgent"
                modelKey="codeFixerModel"
                optional={false}
                {...cardProps}
              />
            </div>
            {reviewerSlots.openCount < 2 && (
              <button
                type="button"
                onClick={reviewerSlots.addSlot}
                className="self-start rounded border border-dashed border-[#2e2e48] px-3 py-1.5 text-xs text-[#6b6b82] hover:border-[#6366f1] hover:text-[#6366f1]"
              >
                + Add Reviewer
              </button>
            )}
          </div>

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
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
