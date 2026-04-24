import type { ProviderInfo } from "../../types";

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
  return sortProvidersByDisplayName(providers).map((p) => ({
    value: p.name,
    label: providerDisplayName(p.name),
  }));
}

/** Returns model options for a provider from the Symphony provider list. */
export function modelOptionsFromProvider(
  provider: ProviderInfo | undefined,
): { value: string; label: string }[] {
  if (!provider) return [];
  return provider.models
    .map((m) => ({ value: m, label: formatModelLabel(m) }))
    .sort((a, b) => a.label.localeCompare(b.label));
}

/** Irregular labels that don't follow the default hyphen-segment rules. */
const SPECIAL_MODEL_LABELS: Record<string, string> = {
  "opus[1m]": "Opus (1M)",
  "kimi-code/kimi-for-coding": "Kimi Code",
};

/** Formats a single hyphen-delimited model-id segment for display.
 *  Numeric version segments ("5.4", "4.6") pass through untouched;
 *  digit+letter codes ("5v") are upper-cased; "1m" becomes "(1M)";
 *  alphabetic tokens are capitalised. */
function formatModelSegment(segment: string): string {
  if (segment === "1m") return "(1M)";
  if (/^\d+[a-z]$/.test(segment)) return segment.toUpperCase();
  if (/^\d/.test(segment)) return segment;
  return segment.charAt(0).toUpperCase() + segment.slice(1);
}

/** Formats a raw model identifier for display.
 *
 *  Derives labels programmatically from the slug so new models format
 *  correctly without code changes. Brand-specific rules:
 *  - ``gpt-<ver>[-<suffix>...]`` → ``GPT-<ver>[ <Suffix>...]`` (e.g. ``gpt-5.5`` → ``GPT-5.5``).
 *  - ``glm-<ver>[-<suffix>...]`` → ``GLM <ver>[ <Suffix>...]``.
 *  - Anything else is split on ``-`` and title-cased segment by segment.
 *  - ``opencode/<slug>`` is treated as ``<slug>``.
 *  - Irregular cases (``opus[1m]``, Kimi alias) live in ``SPECIAL_MODEL_LABELS``.
 */
export function formatModelLabel(model: string): string {
  const special = SPECIAL_MODEL_LABELS[model];
  if (special) return special;

  const slug = model.replace(/^opencode\//, "");
  const [head, ...rest] = slug.split("-");
  if (!head) return model;

  const tail = rest.map(formatModelSegment);

  if (head === "gpt") {
    const [version, ...suffix] = tail;
    if (!version) return "GPT";
    return suffix.length > 0 ? `GPT-${version} ${suffix.join(" ")}` : `GPT-${version}`;
  }

  if (head === "glm") {
    return tail.length > 0 ? `GLM ${tail.join(" ")}` : "GLM";
  }

  return [formatModelSegment(head), ...tail].join(" ");
}

/** Formats the selected assistant as a single provider + model label. */
export function formatAssistantLabel(provider: string, model: string): string {
  const providerLabel = providerDisplayName(provider);
  const modelLabel = formatModelLabel(model);

  if (modelLabel.toLowerCase().startsWith(providerLabel.toLowerCase())) {
    return modelLabel;
  }

  return `${providerLabel} ${modelLabel}`;
}

/** Swarm prompt prefix prepended to the user's message when swarm mode is active. */
export const KIMI_SWARM_PROMPT_PREFIX =
  "[SWARM MODE] You have CreateSubagent and Task tools available. " +
  "Before starting work, analyse the project and create specialised " +
  "subagents (e.g. coder, reviewer, researcher, tester). Then use " +
  "Task to dispatch independent subtasks in parallel — call Task " +
  "multiple times in a single response for maximum concurrency. " +
  "Aim for at least 5 parallel subagents when possible.\n\n" +
  "USER REQUEST:\n";

/** Thinking / reasoning effort options per provider (or per provider:model). */
export const THINKING_OPTIONS: Record<string, { value: string; label: string }[]> = {
  claude: [
    { value: "", label: "Default (High)" },
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
    { value: "max", label: "Max" },
  ],
  "claude:sonnet": [
    { value: "", label: "Default (High)" },
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
  ],
  "claude:haiku": [],
  codex: [
    { value: "", label: "Default (Medium)" },
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
    { value: "xhigh", label: "Extra High" },
  ],
  kimi: [
    { value: "", label: "Default (On)" },
    { value: "on", label: "On" },
    { value: "off", label: "Off" },
  ],
};

/** Returns thinking options for a specific provider and model.
 *  Checks for a model-specific override first, then falls back to
 *  the provider default. Returns undefined when no options apply. */
export function getThinkingOptions(
  provider: string,
  model: string,
): { value: string; label: string }[] | undefined {
  const modelKey = `${provider}:${model}`;
  if (modelKey in THINKING_OPTIONS) {
    const options = THINKING_OPTIONS[modelKey];
    return options.length > 0 ? options : undefined;
  }
  return THINKING_OPTIONS[provider];
}

/** Kimi swarm mode options. */
export const SWARM_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "Disabled" },
  { value: "enabled", label: "Enabled" },
];

/** Kimi Ralph Loop iteration options (only shown when swarm is enabled). */
export const RALPH_ITERATIONS_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "Default (1)" },
  { value: "1", label: "1" },
  { value: "3", label: "3" },
  { value: "5", label: "5" },
  { value: "-1", label: "Unlimited (Agent Decides)" },
];

export const RALPH_TRIGGER_LABELS: Record<string, string> = {
  "": "Default",
};

/** Short labels for the trigger button, keyed by option value.
 *  The menu still shows the full label from THINKING_OPTIONS. */
export const THINKING_TRIGGER_LABELS: Record<string, Record<string, string>> = {
  claude: { "": "Default" },
  codex: { "": "Default" },
  kimi: { "": "Default" },
};

/** Returns providers sorted by their display label. */
export function sortProvidersByDisplayName(providers: ProviderInfo[]): ProviderInfo[] {
  return [...providers].sort((a, b) => (
    providerDisplayName(a.name).localeCompare(providerDisplayName(b.name))
  ));
}
