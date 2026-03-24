import type { AppSettings } from "../types";

export const MINIMUM_AGENT_FIELDS = [
  "promptEnhancerAgent",
  "plannerAgent",
  "planAuditorAgent",
  "coderAgent",
  "finalJudgeAgent",
  "executiveSummaryAgent",
] as const;

const MINIMUM_AGENT_LABELS: Record<(typeof MINIMUM_AGENT_FIELDS)[number], string> = {
  promptEnhancerAgent: "Prompt Enhancer",
  plannerAgent: "Planner / Reviewer",
  planAuditorAgent: "Auditor / Merger",
  coderAgent: "Coder / Fixer",
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

/** Keys for agent backend fields in AppSettings (primary stages only). */
type AgentBackendKey = keyof Pick<
  AppSettings,
  | "promptEnhancerAgent"
  | "skillSelectorAgent"
  | "plannerAgent"
  | "planAuditorAgent"
  | "coderAgent"
  | "codeReviewerAgent"
  | "reviewMergerAgent"
  | "codeFixerAgent"
  | "finalJudgeAgent"
  | "executiveSummaryAgent"
>;

/** Keys for per-stage model fields in AppSettings (primary stages only). */
type AgentModelKey = keyof Pick<
  AppSettings,
  | "promptEnhancerModel"
  | "skillSelectorModel"
  | "plannerModel"
  | "planAuditorModel"
  | "coderModel"
  | "codeReviewerModel"
  | "reviewMergerModel"
  | "codeFixerModel"
  | "finalJudgeModel"
  | "executiveSummaryModel"
>;

type AgentStageBinding = {
  backendKey: AgentBackendKey;
  modelKey: AgentModelKey;
  optional: boolean;
};

const AGENT_STAGE_BINDINGS: AgentStageBinding[] = [
  { backendKey: "promptEnhancerAgent", modelKey: "promptEnhancerModel", optional: false },
  { backendKey: "skillSelectorAgent", modelKey: "skillSelectorModel", optional: true },
  { backendKey: "plannerAgent", modelKey: "plannerModel", optional: true },
  { backendKey: "planAuditorAgent", modelKey: "planAuditorModel", optional: true },
  { backendKey: "coderAgent", modelKey: "coderModel", optional: false },
  { backendKey: "codeReviewerAgent", modelKey: "codeReviewerModel", optional: false },
  { backendKey: "reviewMergerAgent", modelKey: "reviewMergerModel", optional: true },
  { backendKey: "codeFixerAgent", modelKey: "codeFixerModel", optional: false },
  { backendKey: "finalJudgeAgent", modelKey: "finalJudgeModel", optional: false },
  { backendKey: "executiveSummaryAgent", modelKey: "executiveSummaryModel", optional: false },
];

function parseEnabledModels(csv: string): string[] {
  return csv
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
}

/** Legacy per-CLI model CSV field mapping. */
const LEGACY_MODEL_CSV_KEY: Record<string, keyof AppSettings> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

/** Returns enabled models for a backend, checking providerModels first then legacy fields. */
function enabledModelsForBackend(settings: AppSettings, backend: string): string[] {
  // Dynamic providerModels takes precedence.
  const dynamic = settings.providerModels?.[backend];
  if (dynamic !== undefined) return parseEnabledModels(dynamic);
  // Fall back to legacy per-CLI fields.
  const legacyKey = LEGACY_MODEL_CSV_KEY[backend];
  if (legacyKey) return parseEnabledModels(settings[legacyKey] as string);
  return [];
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
 * - Also sanitises extra planner/reviewer arrays.
 */
export function sanitiseAgentAssignmentsForEnabledModels(settings: AppSettings): AppSettings {
  const next: AppSettings = { ...settings };

  for (const binding of AGENT_STAGE_BINDINGS) {
    const backend = next[binding.backendKey] as string | null;
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

  // Sanitise extra planner/reviewer arrays.
  next.extraPlanners = sanitiseExtraSlots(next, next.extraPlanners);
  next.extraReviewers = sanitiseExtraSlots(next, next.extraReviewers);

  return next;
}

function sanitiseExtraSlots(
  settings: AppSettings,
  slots: AppSettings["extraPlanners"],
): AppSettings["extraPlanners"] {
  return slots.map((slot) => {
    if (!slot.agent) {
      return { agent: null, model: null };
    }
    const enabled = enabledModelsForBackend(settings, slot.agent);
    if (enabled.length === 0) {
      return { agent: null, model: null };
    }
    if (!slot.model || !enabled.includes(slot.model)) {
      return { agent: slot.agent, model: enabled[0] };
    }
    return slot;
  });
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
