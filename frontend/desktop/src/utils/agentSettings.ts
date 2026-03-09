import type { AgentBackend, AppSettings } from "../types";

export const MINIMUM_AGENT_FIELDS = [
  "promptEnhancerAgent",
  "generatorAgent",
  "reviewerAgent",
  "fixerAgent",
  "finalJudgeAgent",
  "executiveSummaryAgent",
] as const;

const MINIMUM_AGENT_LABELS: Record<(typeof MINIMUM_AGENT_FIELDS)[number], string> = {
  promptEnhancerAgent: "Prompt Enhancer",
  generatorAgent: "Coder",
  reviewerAgent: "Code Reviewer",
  fixerAgent: "Code Fixer",
  finalJudgeAgent: "Judge",
  executiveSummaryAgent: "Executive Summary",
};

export function missingMinimumAgentLabels(settings: AppSettings): string[] {
  return MINIMUM_AGENT_FIELDS
    .filter((field) => !settings[field])
    .map((field) => MINIMUM_AGENT_LABELS[field]);
}

export function hasMinimumAgentsConfigured(settings: AppSettings): boolean {
  return missingMinimumAgentLabels(settings).length === 0;
}

type AgentStageBinding = {
  backendKey: keyof Pick<
    AppSettings,
    | "promptEnhancerAgent"
    | "skillSelectorAgent"
    | "plannerAgent"
    | "planAuditorAgent"
    | "generatorAgent"
    | "reviewerAgent"
    | "fixerAgent"
    | "finalJudgeAgent"
    | "executiveSummaryAgent"
  >;
  modelKey: keyof Pick<
    AppSettings,
    | "promptEnhancerModel"
    | "skillSelectorModel"
    | "plannerModel"
    | "planAuditorModel"
    | "generatorModel"
    | "reviewerModel"
    | "fixerModel"
    | "finalJudgeModel"
    | "executiveSummaryModel"
  >;
  optional: boolean;
};

const AGENT_STAGE_BINDINGS: AgentStageBinding[] = [
  { backendKey: "promptEnhancerAgent", modelKey: "promptEnhancerModel", optional: false },
  { backendKey: "skillSelectorAgent", modelKey: "skillSelectorModel", optional: true },
  { backendKey: "plannerAgent", modelKey: "plannerModel", optional: true },
  { backendKey: "planAuditorAgent", modelKey: "planAuditorModel", optional: true },
  { backendKey: "generatorAgent", modelKey: "generatorModel", optional: false },
  { backendKey: "reviewerAgent", modelKey: "reviewerModel", optional: false },
  { backendKey: "fixerAgent", modelKey: "fixerModel", optional: false },
  { backendKey: "finalJudgeAgent", modelKey: "finalJudgeModel", optional: false },
  { backendKey: "executiveSummaryAgent", modelKey: "executiveSummaryModel", optional: false },
];

function parseEnabledModels(csv: string): string[] {
  return csv
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
}

function enabledModelsForBackend(settings: AppSettings, backend: AgentBackend): string[] {
  switch (backend) {
    case "claude":
      return parseEnabledModels(settings.claudeModel);
    case "codex":
      return parseEnabledModels(settings.codexModel);
    case "gemini":
      return parseEnabledModels(settings.geminiModel);
    case "kimi":
      return parseEnabledModels(settings.kimiModel);
    case "opencode":
      return parseEnabledModels(settings.opencodeModel);
    default:
      return [];
  }
}

function clearStageAssignment(
  next: AppSettings,
  binding: AgentStageBinding,
): void {
  next[binding.backendKey] = null;
  setStageModel(next, binding, null);
}

function setStageModel(
  next: AppSettings,
  binding: AgentStageBinding,
  value: string | null,
): void {
  (next as unknown as Record<string, string | null>)[binding.modelKey] = binding.optional
    ? value
    : (value ?? "");
}

/**
 * Keeps agent stage assignments consistent with enabled CLI models.
 *
 * Rules:
 * - If a stage backend has no enabled models, clear backend + model.
 * - If a stage model is no longer enabled for its backend, pick the first enabled model.
 * - If backend is unset, clear model value.
 */
export function sanitiseAgentAssignmentsForEnabledModels(settings: AppSettings): AppSettings {
  const next: AppSettings = { ...settings };

  for (const binding of AGENT_STAGE_BINDINGS) {
    const backend = next[binding.backendKey] as AgentBackend | null;
    if (!backend) {
      setStageModel(next, binding, null);
      continue;
    }

    const enabledModels = enabledModelsForBackend(next, backend);
    if (enabledModels.length === 0) {
      clearStageAssignment(next, binding);
      continue;
    }

    const modelValue = next[binding.modelKey] as string | null;
    if (!modelValue || !enabledModels.includes(modelValue)) {
      setStageModel(next, binding, enabledModels[0]);
    }
  }

  return next;
}

export function missingMinimumAgentModelLabels(settings: AppSettings): string[] {
  const sanitised = sanitiseAgentAssignmentsForEnabledModels(settings);
  const missing: string[] = [];

  for (const field of MINIMUM_AGENT_FIELDS) {
    if (!sanitised[field]) {
      missing.push(MINIMUM_AGENT_LABELS[field]);
    }
  }

  return missing;
}
