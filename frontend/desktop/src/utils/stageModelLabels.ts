import type { AppSettings, AgentBackend, PipelineStage } from "../types";

function formatModelLabel(agent: AgentBackend | null | undefined, model: string | null | undefined): string | undefined {
  const cleanModel = model?.trim();
  if (agent && cleanModel) return `${agent} · ${cleanModel}`;
  if (agent) return agent;
  if (cleanModel) return cleanModel;
  return undefined;
}

/** Returns the configured model label for pipeline stages shown in custom cards. */
export function stageModelLabel(stage: PipelineStage, settings: AppSettings | null): string | undefined {
  if (!settings) return undefined;
  if (stage === "prompt_enhance") return formatModelLabel(settings.promptEnhancerAgent, settings.promptEnhancerModel);
  if (stage === "plan") return formatModelLabel(settings.plannerAgent, settings.plannerModel);
  if (stage === "plan_audit") return formatModelLabel(settings.planAuditorAgent, settings.planAuditorModel);
  if (stage === "coder") return formatModelLabel(settings.coderAgent, settings.coderModel);
  if (stage === "code_reviewer") return formatModelLabel(settings.codeReviewerAgent, settings.codeReviewerModel);
  if (stage === "code_fixer") return formatModelLabel(settings.codeFixerAgent, settings.codeFixerModel);

  // Dynamic extra planner: plan2, plan3, plan4, ...
  const planMatch = /^plan(\d+)$/.exec(stage);
  if (planMatch) {
    const idx = parseInt(planMatch[1], 10) - 2;
    const slot = settings.extraPlanners[idx];
    if (slot) return formatModelLabel(slot.agent, slot.model);
    return undefined;
  }

  // Dynamic extra reviewer: code_reviewer2, code_reviewer3, ...
  const reviewMatch = /^code_reviewer(\d+)$/.exec(stage);
  if (reviewMatch) {
    const idx = parseInt(reviewMatch[1], 10) - 2;
    const slot = settings.extraReviewers[idx];
    if (slot) return formatModelLabel(slot.agent, slot.model);
    return undefined;
  }

  return undefined;
}
