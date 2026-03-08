import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, AgentBackend } from "../types";
import { CLI_MODEL_OPTIONS } from "../types";

interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
}

/** Agent backend options for dropdown selects. */
const BACKEND_OPTIONS: { value: AgentBackend; label: string }[] = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini" },
];

/** Settings key for each CLI's enabled-models field (comma-separated). */
type ModelCsvKey = "claudeModel" | "codexModel" | "geminiModel";

/** Map from backend name to its comma-separated enabled-models settings key. */
const BACKEND_CSV_KEY: Record<AgentBackend, ModelCsvKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
};

/** Parses a comma-separated model string into an array. */
function parseEnabledModels(csv: string): string[] {
  return csv.split(",").map((s) => s.trim()).filter(Boolean);
}

/** Returns the enabled model options for a given backend, filtered by enabled models in settings. */
function getModelOptionsForBackend(
  backend: AgentBackend,
  settings: AppSettings,
): { value: string; label: string }[] {
  const csvKey = BACKEND_CSV_KEY[backend];
  const enabled = new Set(parseEnabledModels(settings[csvKey]));
  const allOptions = CLI_MODEL_OPTIONS[backend] ?? [];
  return allOptions.filter((opt) => enabled.has(opt.value));
}

/** Reusable row: backend select + model select side-by-side. */
function AgentRow({
  label,
  backend,
  model,
  settings,
  onBackendChange,
  onModelChange,
}: {
  label: string;
  backend: AgentBackend;
  model: string;
  settings: AppSettings;
  onBackendChange: (v: AgentBackend) => void;
  onModelChange: (v: string) => void;
}): ReactNode {
  const modelOptions = getModelOptionsForBackend(backend, settings);

  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#9898b0]">{label}</span>
      <div className="flex gap-2">
        <select
          value={backend}
          onChange={(e) => onBackendChange(e.target.value as AgentBackend)}
          className="flex-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
        >
          {BACKEND_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          value={model}
          onChange={(e) => onModelChange(e.target.value)}
          className="flex-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
        >
          {modelOptions.length === 0 && (
            <option value="">No models enabled</option>
          )}
          {modelOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

/** Reusable optional row with a skip option. */
function OptionalAgentRow({
  label,
  backend,
  model,
  settings,
  onBackendChange,
  onModelChange,
}: {
  label: string;
  backend: AgentBackend | null;
  model: string | null;
  settings: AppSettings;
  onBackendChange: (v: AgentBackend | null) => void;
  onModelChange: (v: string | null) => void;
}): ReactNode {
  const modelOptions = backend
    ? getModelOptionsForBackend(backend, settings)
    : [];

  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#9898b0]">{label}</span>
      <div className="flex gap-2">
        <select
          value={backend ?? ""}
          onChange={(e) => {
            const val = e.target.value === "" ? null : (e.target.value as AgentBackend);
            onBackendChange(val);
            // Auto-select first enabled model when backend changes
            if (val) {
              const opts = getModelOptionsForBackend(val, settings);
              onModelChange(opts.length > 0 ? opts[0].value : null);
            } else {
              onModelChange(null);
            }
          }}
          className="flex-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
        >
          <option value="">Not selected (Skip)</option>
          {BACKEND_OPTIONS.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <select
          value={model ?? ""}
          onChange={(e) => onModelChange(e.target.value || null)}
          disabled={!backend}
          className="flex-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none disabled:cursor-not-allowed disabled:opacity-50"
        >
          {!backend && <option value="">—</option>}
          {modelOptions.length === 0 && backend && (
            <option value="">No models enabled</option>
          )}
          {modelOptions.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({ settings, onSave }: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    setDraft((prev) => ({ ...prev, ...patch }));
  }

  /** When a backend changes for a required stage, auto-select the first enabled model. */
  function handleBackendChange(
    backendKey: keyof AppSettings,
    modelKey: keyof AppSettings,
    newBackend: AgentBackend,
  ): void {
    const opts = getModelOptionsForBackend(newBackend, draft);
    const firstModel = opts.length > 0 ? opts[0].value : "";
    update({ [backendKey]: newBackend, [modelKey]: firstModel });
  }

  function handleSave(): void {
    onSave(draft);
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-lg flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>

          {/* Agent role mapping */}
          <div className="flex flex-col gap-3">
            <AgentRow
              label="Prompt Enhancer"
              backend={draft.promptEnhancerAgent}
              model={draft.promptEnhancerModel}
              settings={draft}
              onBackendChange={(v) => handleBackendChange("promptEnhancerAgent", "promptEnhancerModel", v)}
              onModelChange={(v) => update({ promptEnhancerModel: v })}
            />
            <OptionalAgentRow
              label="Planner"
              backend={draft.plannerAgent}
              model={draft.plannerModel}
              settings={draft}
              onBackendChange={(v) => update({ plannerAgent: v })}
              onModelChange={(v) => update({ plannerModel: v })}
            />
            <OptionalAgentRow
              label="Plan Auditor"
              backend={draft.planAuditorAgent}
              model={draft.planAuditorModel}
              settings={draft}
              onBackendChange={(v) => update({ planAuditorAgent: v })}
              onModelChange={(v) => update({ planAuditorModel: v })}
            />
            <AgentRow
              label="Coder"
              backend={draft.generatorAgent}
              model={draft.generatorModel}
              settings={draft}
              onBackendChange={(v) => handleBackendChange("generatorAgent", "generatorModel", v)}
              onModelChange={(v) => update({ generatorModel: v })}
            />
            <AgentRow
              label="Code Reviewer / Auditor"
              backend={draft.reviewerAgent}
              model={draft.reviewerModel}
              settings={draft}
              onBackendChange={(v) => handleBackendChange("reviewerAgent", "reviewerModel", v)}
              onModelChange={(v) => update({ reviewerModel: v })}
            />
            <AgentRow
              label="Code Fixer"
              backend={draft.fixerAgent}
              model={draft.fixerModel}
              settings={draft}
              onBackendChange={(v) => handleBackendChange("fixerAgent", "fixerModel", v)}
              onModelChange={(v) => update({ fixerModel: v })}
            />
            <AgentRow
              label="Judge"
              backend={draft.finalJudgeAgent}
              model={draft.finalJudgeModel}
              settings={draft}
              onBackendChange={(v) => handleBackendChange("finalJudgeAgent", "finalJudgeModel", v)}
              onModelChange={(v) => update({ finalJudgeModel: v })}
            />
          </div>

          {/* Pipeline parameters */}
          <div className="flex flex-col gap-3 border-t border-[#2e2e48] pt-4">
            <span className="text-sm font-medium text-[#e4e4ed]">Pipeline</span>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Max Iterations</span>
              <input
                type="number"
                min={1}
                max={10}
                value={draft.maxIterations}
                onChange={(e) => update({ maxIterations: Math.max(1, Math.min(10, Number(e.target.value))) })}
                className="w-20 rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
            </label>
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={draft.requireGit}
                onChange={(e) => update({ requireGit: e.target.checked })}
                className="rounded border-[#2e2e48] accent-[#6366f1]"
              />
              <span className="text-xs text-[#9898b0]">Require Git repository</span>
            </label>
          </div>

          {/* Save */}
          <button
            onClick={handleSave}
            className="self-start rounded bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] hover:bg-white transition-colors"
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
