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
  if (stage === "plan2") return formatModelLabel(settings.planner2Agent, settings.planner2Model);
  if (stage === "plan3") return formatModelLabel(settings.planner3Agent, settings.planner3Model);
  if (stage === "plan_audit") return formatModelLabel(settings.planAuditorAgent, settings.planAuditorModel);
  if (stage === "coder") return formatModelLabel(settings.coderAgent, settings.coderModel);
  if (stage === "code_reviewer") return formatModelLabel(settings.codeReviewerAgent, settings.codeReviewerModel);
  if (stage === "code_reviewer2") return formatModelLabel(settings.codeReviewer2Agent, settings.codeReviewer2Model);
  if (stage === "code_reviewer3") return formatModelLabel(settings.codeReviewer3Agent, settings.codeReviewer3Model);
  if (stage === "code_fixer") return formatModelLabel(settings.codeFixerAgent, settings.codeFixerModel);
  return undefined;
}
