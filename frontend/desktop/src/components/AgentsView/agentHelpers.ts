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

/** Returns enabled model CSV for a backend, checking providerModels first then legacy fields. */
function enabledModelsCsvForBackend(
  backend: string,
  settings: AppSettings,
): string {
  // Dynamic providerModels takes precedence.
  const dynamic = settings.providerModels?.[backend];
  if (dynamic !== undefined) return dynamic;
  // Fall back to legacy per-CLI fields.
  const legacyKey = LEGACY_MODEL_CSV_KEY[backend];
  if (legacyKey) return settings[legacyKey] as string;
  return "";
}

/**
 * Returns enabled model options for a given backend, filtered by what
 * the user has enabled in settings and intersected with what the
 * provider actually offers.
 */
export function getModelOptionsForBackend(
  backend: string,
  settings: AppSettings,
  providers: ProviderInfo[],
): { value: string; label: string }[] {
  const enabled = new Set(parseEnabledModels(enabledModelsCsvForBackend(backend, settings)));
  const provider = providers.find((p) => p.name === backend);
  const allOptions = modelOptionsFromProvider(provider);
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
