import type { ReactNode } from "react";
import { useState, useEffect, useRef } from "react";
import type { AppSettings, CliHealth } from "../../types";
import { sanitiseAgentAssignmentsForEnabledModels } from "../../utils/agentSettings";
import { InlineStageSlot } from "./InlineStageSlot";
import { StageCard } from "./StageCard";

/** Props for the AgentsView component. */
export interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  cliHealth?: CliHealth | null;
  cliHealthChecking?: boolean;
}

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({
  settings, onSave, cliHealth, cliHealthChecking,
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

  function handleFreshStart(): void {
    const cleared: AppSettings = {
      ...draftRef.current,
      promptEnhancerAgent: null, skillSelectorAgent: null,
      plannerAgent: null, planner2Agent: null, planner3Agent: null,
      planAuditorAgent: null, coderAgent: null,
      codeReviewerAgent: null, codeReviewer2Agent: null, codeReviewer3Agent: null,
      reviewMergerAgent: null, codeFixerAgent: null,
      finalJudgeAgent: null, executiveSummaryAgent: null,
      promptEnhancerModel: "", skillSelectorModel: null,
      plannerModel: null, planner2Model: null, planner3Model: null,
      planAuditorModel: null, coderModel: "",
      codeReviewerModel: "", codeReviewer2Model: null, codeReviewer3Model: null,
      reviewMergerModel: null, codeFixerModel: "",
      finalJudgeModel: "", executiveSummaryModel: "",
    };
    draftRef.current = cleared;
    setDraft(cleared);
    onSave(cleared);
  }

  const health = cliHealth ?? null;
  const checking = Boolean(cliHealthChecking);
  const plannerCount = [draft.plannerAgent, draft.planner2Agent, draft.planner3Agent].filter(Boolean).length;
  const reviewerCount = [draft.codeReviewerAgent, draft.codeReviewer2Agent, draft.codeReviewer3Agent].filter(Boolean).length;

  // Track which extra slots are visible (separate from whether an agent is selected).
  // A slot is visible if it was opened by the user OR already has an agent configured.
  const [extraSlots, setExtraSlots] = useState({
    planner2: draft.planner2Agent !== null,
    planner3: draft.planner3Agent !== null,
    reviewer2: draft.codeReviewer2Agent !== null,
    reviewer3: draft.codeReviewer3Agent !== null,
  });

  // Sync slot visibility when settings change externally.
  // Only *open* slots that have an agent configured — never close slots the user opened manually.
  useEffect(() => {
    setExtraSlots((prev) => ({
      planner2: prev.planner2 || settings.planner2Agent !== null,
      planner3: prev.planner3 || settings.planner3Agent !== null,
      reviewer2: prev.reviewer2 || settings.codeReviewer2Agent !== null,
      reviewer3: prev.reviewer3 || settings.codeReviewer3Agent !== null,
    }));
  }, [settings]);

  const plannerSlotsOpen = (extraSlots.planner2 ? 1 : 0) + (extraSlots.planner3 ? 1 : 0);
  const reviewerSlotsOpen = (extraSlots.reviewer2 ? 1 : 0) + (extraSlots.reviewer3 ? 1 : 0);

  function addPlannerSlot(): void {
    if (!extraSlots.planner2) setExtraSlots((s) => ({ ...s, planner2: true }));
    else if (!extraSlots.planner3) setExtraSlots((s) => ({ ...s, planner3: true }));
  }

  function removePlannerSlot(slot: "planner2" | "planner3"): void {
    if (slot === "planner2" && extraSlots.planner3) {
      setExtraSlots((s) => ({ ...s, planner3: false }));
      update({
        planner2Agent: draftRef.current.planner3Agent,
        planner2Model: draftRef.current.planner3Model,
        planner3Agent: null,
        planner3Model: null,
      });
      return;
    }
    setExtraSlots((s) => ({ ...s, [slot]: false }));
    if (slot === "planner2") update({ planner2Agent: null, planner2Model: null });
    else update({ planner3Agent: null, planner3Model: null });
  }

  function addReviewerSlot(): void {
    if (!extraSlots.reviewer2) setExtraSlots((s) => ({ ...s, reviewer2: true }));
    else if (!extraSlots.reviewer3) setExtraSlots((s) => ({ ...s, reviewer3: true }));
  }

  function removeReviewerSlot(slot: "reviewer2" | "reviewer3"): void {
    if (slot === "reviewer2" && extraSlots.reviewer3) {
      setExtraSlots((s) => ({ ...s, reviewer3: false }));
      update({
        codeReviewer2Agent: draftRef.current.codeReviewer3Agent,
        codeReviewer2Model: draftRef.current.codeReviewer3Model,
        codeReviewer3Agent: null,
        codeReviewer3Model: null,
      });
      return;
    }
    setExtraSlots((s) => ({ ...s, [slot]: false }));
    if (slot === "reviewer2") update({ codeReviewer2Agent: null, codeReviewer2Model: null });
    else update({ codeReviewer3Agent: null, codeReviewer3Model: null });
  }

  const cardProps = { draft, cliHealth: health, cliHealthChecking: checking, onUpdate: update };

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>

          <div className="flex items-center gap-3">
            <button onClick={handleFreshStart}
              className="rounded border border-[#2e2e48] bg-[#24243a] px-3 py-1.5 text-xs text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed]">
              Fresh Start: Clear All
            </button>
          </div>

          {/* Pre-planning */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            <StageCard label="Prompt Enhancer" tag="(required)"
              backendKey="promptEnhancerAgent" modelKey="promptEnhancerModel"
              optional={false} {...cardProps} />
            <StageCard label="Skill Selector" tag="(optional)"
              backendKey="skillSelectorAgent" modelKey="skillSelectorModel"
              optional={true} {...cardProps} />
          </div>

          {/* Planning section */}
          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium text-[#6b6b82] uppercase tracking-wider">
              Planning
            </span>
            <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
              <StageCard label="Planner 1" tag="(optional)"
                backendKey="plannerAgent" modelKey="plannerModel"
                optional={true} {...cardProps}>
                {extraSlots.planner2 && (
                  <InlineStageSlot
                    label="Planner 2"
                    backendKey="planner2Agent"
                    modelKey="planner2Model"
                    optional={true}
                    onRemove={() => removePlannerSlot("planner2")}
                    {...cardProps}
                  />
                )}
                {extraSlots.planner3 && (
                  <InlineStageSlot
                    label="Planner 3"
                    backendKey="planner3Agent"
                    modelKey="planner3Model"
                    optional={true}
                    onRemove={() => removePlannerSlot("planner3")}
                    {...cardProps}
                  />
                )}
              </StageCard>
              {plannerCount > 0 && (
                <StageCard label="Plan Auditor" tag={plannerCount > 1 ? "(auto — merges & audits)" : "(auto — audits plan)"}
                  backendKey="planAuditorAgent" modelKey="planAuditorModel"
                  optional={true} {...cardProps} />
              )}
            </div>
            {plannerSlotsOpen < 2 && (
              <button onClick={addPlannerSlot}
                className="self-start rounded border border-dashed border-[#2e2e48] px-3 py-1.5 text-xs text-[#6b6b82] hover:border-[#6366f1] hover:text-[#6366f1]">
                + Add Planner
              </button>
            )}
          </div>

          {/* Coding */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            <StageCard label="Coder" tag="(required)"
              backendKey="coderAgent" modelKey="coderModel"
              optional={false} {...cardProps} />
          </div>

          {/* Review section */}
          <div className="flex flex-col gap-3">
            <span className="text-xs font-medium text-[#6b6b82] uppercase tracking-wider">Review</span>
            <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
              <StageCard label="Reviewer 1" tag="(required)"
                backendKey="codeReviewerAgent" modelKey="codeReviewerModel"
                optional={false} {...cardProps}>
                {extraSlots.reviewer2 && (
                  <InlineStageSlot
                    label="Reviewer 2"
                    backendKey="codeReviewer2Agent"
                    modelKey="codeReviewer2Model"
                    optional={true}
                    onRemove={() => removeReviewerSlot("reviewer2")}
                    {...cardProps}
                  />
                )}
                {extraSlots.reviewer3 && (
                  <InlineStageSlot
                    label="Reviewer 3"
                    backendKey="codeReviewer3Agent"
                    modelKey="codeReviewer3Model"
                    optional={true}
                    onRemove={() => removeReviewerSlot("reviewer3")}
                    {...cardProps}
                  />
                )}
              </StageCard>
              <StageCard label="Code Fixer" tag="(required)"
                backendKey="codeFixerAgent" modelKey="codeFixerModel"
                optional={false} {...cardProps} />
            </div>
            {reviewerCount >= 2 && (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
                <StageCard label="Review Merger" tag="(auto — combines reviews)"
                  backendKey="reviewMergerAgent" modelKey="reviewMergerModel"
                  optional={true} {...cardProps} />
              </div>
            )}
            {reviewerSlotsOpen < 2 && (
              <button onClick={addReviewerSlot}
                className="self-start rounded border border-dashed border-[#2e2e48] px-3 py-1.5 text-xs text-[#6b6b82] hover:border-[#6366f1] hover:text-[#6366f1]">
                + Add Reviewer
              </button>
            )}
          </div>

          {/* Judgement */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            <StageCard label="Judge" tag="(required)"
              backendKey="finalJudgeAgent" modelKey="finalJudgeModel"
              optional={false} {...cardProps} />
            <StageCard label="Executive Summary" tag="(required)"
              backendKey="executiveSummaryAgent" modelKey="executiveSummaryModel"
              optional={false} {...cardProps} />
          </div>

          {/* Pipeline parameters */}
          <div className="flex flex-col gap-3 border-t border-[#2e2e48] pt-4">
            <span className="text-sm font-medium text-[#e4e4ed]">Pipeline</span>
            <div className="grid grid-cols-2 gap-4">
              <label className="flex flex-col gap-1">
                <span className="text-xs font-medium text-[#9898b0]">Max Iterations</span>
                <input type="number" min={1} max={10} value={draft.maxIterations}
                  onChange={(e) => update({ maxIterations: Math.max(1, Math.min(10, Number(e.target.value))) })}
                  className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#3e3e58] focus:outline-none" />
              </label>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
