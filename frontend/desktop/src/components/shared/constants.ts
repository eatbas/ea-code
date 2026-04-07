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

/** Formats the selected assistant as a single provider + model label. */
export function formatAssistantLabel(provider: string, model: string): string {
  const providerLabel = providerDisplayName(provider);
  const modelLabel = formatModelLabel(model);

  if (modelLabel.toLowerCase().startsWith(providerLabel.toLowerCase())) {
    return modelLabel;
  }

  return `${providerLabel} ${modelLabel}`;
}

/** Thinking / reasoning effort options per provider. */
export const THINKING_OPTIONS: Record<string, { value: string; label: string }[]> = {
  claude: [
    { value: "", label: "Default (High)" },
    { value: "low", label: "Low" },
    { value: "medium", label: "Medium" },
    { value: "high", label: "High" },
    { value: "max", label: "Max" },
  ],
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
