import type { PipelineStage, ProviderInfo } from "../../types";

/** Capitalises a provider name for display (e.g. "claude" → "Claude", "opencode" → "OpenCode"). */
export function providerDisplayName(name: string): string {
  const known: Record<string, string> = {
    claude: "Claude",
    codex: "Codex",
    gemini: "Gemini",
    kimi: "Kimi",
    opencode: "OpenCode",
    copilot: "Copilot",
  };
  return known[name] ?? name.charAt(0).toUpperCase() + name.slice(1);
}

/** Derives backend options from dynamic provider list. */
export function backendOptionsFromProviders(
  providers: ProviderInfo[],
): { value: string; label: string }[] {
  return providers.map((p) => ({ value: p.name, label: providerDisplayName(p.name) }));
}

/** Returns model options for a provider from the hive-api provider list. */
export function modelOptionsFromProvider(
  provider: ProviderInfo | undefined,
): { value: string; label: string }[] {
  if (!provider) return [];
  return provider.models.map((m) => ({ value: m, label: formatModelLabel(m) }));
}

/** Formats a raw model identifier for display. */
function formatModelLabel(model: string): string {
  const known: Record<string, string> = {
    sonnet: "Sonnet",
    opus: "Opus",
    haiku: "Haiku",
    "gpt-5.3-codex": "GPT-5.3 Codex",
    "gpt-5.4": "GPT-5.4",
    "gpt-5.4-mini": "GPT-5.4 Mini",
    "gemini-3-flash-preview": "Gemini 3 Flash",
    "gemini-3.1-pro-preview": "Gemini 3.1 Pro",
    "kimi-code/kimi-for-coding": "Kimi Code",
    "opencode/glm-5": "GLM 5",
    "opencode/glm-5-turbo": "GLM 5 Turbo",
    "opencode/glm-4.7": "GLM 4.7",
    "claude-sonnet-4.6": "Claude Sonnet 4.6",
    "claude-haiku-4.5": "Claude Haiku 4.5",
  };
  return known[model] ?? model;
}

const KNOWN_STAGE_LABELS: Record<string, string> = {
  prompt_enhance: "Enhancing Prompt",
  skill_select: "Skills",
  plan: "Planning",
  plan_audit: "Auditing Plan",
  coder: "Coding",
  code_reviewer: "Code Review",
  review_merge: "Merging Reviews",
  code_fixer: "Code Fix",
  judge: "Judge",
  executive_summary: "Summary",
  direct_task: "Direct Task",
};

/** Display label for a pipeline stage. */
export function stageLabel(stage: PipelineStage): string {
  const known = KNOWN_STAGE_LABELS[stage];
  if (known) return known;
  // Dynamic planner: plan2 → "Planning (P2)", plan3 → "Planning (P3)", ...
  const planMatch = /^plan(\d+)$/.exec(stage);
  if (planMatch) return `Planning (P${planMatch[1]})`;
  // Dynamic reviewer: code_reviewer2 → "Code Review (R2)", ...
  const reviewMatch = /^code_reviewer(\d+)$/.exec(stage);
  if (reviewMatch) return `Code Review (R${reviewMatch[1]})`;
  return stage;
}

const KNOWN_BADGE_CLASSES: Record<string, string> = {
  prompt_enhance: "bg-[#64c8ff]/20",
  skill_select: "bg-[#46cd7d]/25",
  plan: "bg-[#40c4ff]/25",
  plan_audit: "bg-[#ffc440]/25",
  coder: "bg-[#5a8cff]/25",
  code_reviewer: "bg-[#ffb432]/20",
  review_merge: "bg-[#ffc440]/25",
  code_fixer: "bg-[#b464ff]/20",
  judge: "bg-[#ff6464]/20",
  executive_summary: "bg-[#00c850]/30",
  direct_task: "bg-[#5a8cff]/25",
};

/** Badge background class for a pipeline stage. */
export function stageBadgeClass(stage: PipelineStage): string {
  const known = KNOWN_BADGE_CLASSES[stage];
  if (known) return known;
  if (/^plan\d+$/.test(stage)) return "bg-[#40c4ff]/25";
  if (/^code_reviewer\d+$/.test(stage)) return "bg-[#ffb432]/20";
  return "bg-[#9898b0]/20";
}

/** Display labels for artifact kinds (kept for runtime artifact display during live runs). */
export const ARTIFACT_LABELS: Record<string, string> = {
  plan: "Plan",
  plan_audit: "Plan Audit",
  plan_final: "Final Plan",
  plan_revised: "Revised Plan",
  review: "Review",
  judge: "Judge Verdict",
  executive_summary: "Summary",
  selected_skills: "Selected Skills",
  workspace_context: "Workspace Context",
  result: "Result",
};
