import type { AppSettings } from "../types";

/** Legacy per-CLI model CSV settings keys. */
const LEGACY_MODEL_KEY: Record<string, keyof AppSettings> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

/** Parse a comma-separated model string into a Set. */
export function parseEnabledModels(csv: string): Set<string> {
  return new Set(csv.split(",").map((s) => s.trim()).filter(Boolean));
}

/** Serialise a Set of model identifiers back to a CSV string. */
export function serialiseEnabledModels(models: Set<string>): string {
  return Array.from(models).join(",");
}

/**
 * Resolve the currently enabled models for a given provider,
 * checking `providerModels` first then falling back to legacy fields.
 */
export function getEnabledModels(
  settings: AppSettings,
  providerName: string,
): Set<string> {
  const dynamic = settings.providerModels?.[providerName];
  if (dynamic !== undefined) return parseEnabledModels(dynamic);
  const legacyKey = LEGACY_MODEL_KEY[providerName];
  if (legacyKey) return parseEnabledModels(settings[legacyKey] as string);
  return new Set();
}

/**
 * Produce an updated `AppSettings` with the given model CSV applied
 * to both the `providerModels` map and, where applicable, the legacy field.
 */
export function applyModelCsv(
  settings: AppSettings,
  providerName: string,
  csv: string,
): AppSettings {
  const legacyKey = LEGACY_MODEL_KEY[providerName];
  if (legacyKey) {
    return {
      ...settings,
      [legacyKey]: csv,
      providerModels: { ...settings.providerModels, [providerName]: csv },
    };
  }
  return {
    ...settings,
    providerModels: { ...settings.providerModels, [providerName]: csv },
  };
}
