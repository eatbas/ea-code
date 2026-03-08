import type { AgentBackend, AppSettings } from "../../types";
import { CLI_MODEL_OPTIONS } from "../../types";
import { BACKEND_OPTIONS } from "../shared/constants";

/** Settings key for each CLI's enabled-models field (comma-separated). */
export type ModelCsvKey =
  | "claudeModel"
  | "codexModel"
  | "geminiModel"
  | "kimiModel"
  | "opencodeModel"
  | "copilotModel";

/** Map from backend name to its comma-separated enabled-models settings key. */
export const BACKEND_CSV_KEY: Record<AgentBackend, ModelCsvKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
  copilot: "copilotModel",
};

/** Parses a comma-separated model string into an array. */
export function parseEnabledModels(csv: string): string[] {
  return csv.split(",").map((s) => s.trim()).filter(Boolean);
}

/** Returns enabled model options for a given backend, filtered by settings. */
export function getModelOptionsForBackend(
  backend: AgentBackend,
  settings: AppSettings,
): { value: string; label: string }[] {
  const csvKey = BACKEND_CSV_KEY[backend];
  const enabled = new Set(parseEnabledModels(settings[csvKey]));
  const allOptions = CLI_MODEL_OPTIONS[backend] ?? [];
  return allOptions.filter((opt) => enabled.has(opt.value));
}

/** Finds the display label for a backend value. */
export function backendLabel(backend: AgentBackend): string {
  return BACKEND_OPTIONS.find((o) => o.value === backend)?.label ?? backend;
}

/** Finds the display label for a model value within a backend. */
export function modelLabel(backend: AgentBackend, model: string): string {
  const allOptions = CLI_MODEL_OPTIONS[backend] ?? [];
  return allOptions.find((o) => o.value === model)?.label ?? model;
}
