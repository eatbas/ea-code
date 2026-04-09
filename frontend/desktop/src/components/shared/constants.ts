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

/** Formats a raw model identifier for display. */
export function formatModelLabel(model: string): string {
  const known: Record<string, string> = {
    // Claude aliases
    sonnet: "Sonnet",
    opus: "Opus",
    "opus[1m]": "Opus (1M)",
    haiku: "Haiku",
    // Codex models (latest generation)
    "gpt-5.3-codex": "GPT-5.3 Codex",
    "gpt-5.4": "GPT-5.4",
    "gpt-5.4-mini": "GPT-5.4 Mini",
    "gpt-5.4-nano": "GPT-5.4 Nano",
    // Gemini models
    "gemini-2.5-flash": "Gemini 2.5 Flash",
    "gemini-3-flash-preview": "Gemini 3 Flash Preview",
    "gemini-3-pro-preview": "Gemini 3 Pro Preview",
    "gemini-3.1-pro-preview": "Gemini 3.1 Pro Preview",
    "gemini-3.1-pro": "Gemini 3.1 Pro",
    "gemini-2.5-pro": "Gemini 2.5 Pro",
    "gemini-3-pro": "Gemini 3 Pro",
    "gemini-3-flash": "Gemini 3 Flash",
    // Kimi models
    "kimi-code/kimi-for-coding": "Kimi Code",
    // Copilot backend models (latest per tier)
    "claude-sonnet-4.6": "Claude Sonnet 4.6",
    "claude-haiku-4.5": "Claude Haiku 4.5",
    "claude-opus-4.6": "Claude Opus 4.6",
    "claude-opus-4.6-1m": "Claude Opus 4.6 (1M)",
    "claude-opus-4.6-fast": "Claude Opus 4.6 Fast",
    // OpenCode GLM models (latest generation)
    "glm-5": "GLM 5",
    "glm-5-turbo": "GLM 5 Turbo",
    "glm-5.1": "GLM 5.1",
    "glm-5v-turbo": "GLM 5V Turbo",
    // OpenCode GLM models (prefixed — from legacy settings)
    "opencode/glm-5": "GLM 5",
    "opencode/glm-5-turbo": "GLM 5 Turbo",
  };
  return known[model] ?? model;
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
