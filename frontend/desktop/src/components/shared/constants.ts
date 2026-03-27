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
