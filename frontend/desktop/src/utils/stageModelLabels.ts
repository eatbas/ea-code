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
  return undefined;
}
