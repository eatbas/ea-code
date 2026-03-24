import type { AppSettings, ProviderInfo } from "../../types";
import {
  providerDisplayName,
  modelOptionsFromProvider,
} from "../shared/constants";

/** Parses a comma-separated model string into an array. */
export function parseEnabledModels(csv: string): string[] {
  return csv.split(",").map((s) => s.trim()).filter(Boolean);
}

/** Settings key for legacy per-CLI model CSV fields. */
const LEGACY_MODEL_CSV_KEY: Record<string, keyof AppSettings> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

/**
 * Returns the enabled model CSV for a backend, or `undefined` when no
 * explicit configuration exists at all.
 *
 * - `providerModels[backend]` takes precedence (can be `""`).
 * - Falls back to the legacy per-CLI field when non-empty.
 * - Returns `undefined` when neither source has an explicit value.
 */
function enabledModelsCsvForBackend(
  backend: string,
  settings: AppSettings,
): string | undefined {
  // Dynamic providerModels takes precedence — even an empty string is explicit.
  const dynamic = settings.providerModels?.[backend];
  if (dynamic !== undefined) return dynamic;
  // Fall back to legacy per-CLI fields (only when non-empty).
  const legacyKey = LEGACY_MODEL_CSV_KEY[backend];
  if (legacyKey) {
    const val = settings[legacyKey] as string;
    if (val) return val;
  }
  return undefined;
}

/**
 * Returns enabled model options for a given backend, filtered by what
 * the user has enabled in settings and intersected with what the
 * provider actually offers.
 *
 * If no models have been explicitly enabled for a provider, all models
 * from the hive-api provider list are shown (sensible default).
 */
export function getModelOptionsForBackend(
  backend: string,
  settings: AppSettings,
  providers: ProviderInfo[],
): { value: string; label: string }[] {
  const csv = enabledModelsCsvForBackend(backend, settings);
  const provider = providers.find((p) => p.name === backend);
  const allOptions = modelOptionsFromProvider(provider);
  // No explicit selection at all → show every provider model as the default.
  if (csv === undefined) return allOptions;
  // Explicit but empty → user cleared all models for this provider.
  if (!csv) return [];
  const enabled = new Set(parseEnabledModels(csv));
  return allOptions.filter((opt) => enabled.has(opt.value));
}

/** Finds the display label for a backend value. */
export function backendLabel(backend: string): string {
  return providerDisplayName(backend);
}

/** Finds the display label for a model value within a backend. */
export function modelLabel(
  backend: string,
  model: string | null,
  providers: ProviderInfo[],
): string {
  if (!model) return "Not selected";
  const provider = providers.find((p) => p.name === backend);
  const allOptions = modelOptionsFromProvider(provider);
  return allOptions.find((o) => o.value === model)?.label ?? model;
}
