import type { ReactNode } from "react";
import { useState, useEffect, useRef, useCallback } from "react";
import type { AppSettings, AgentBackend } from "../types";
import { CLI_MODEL_OPTIONS } from "../types";

interface AgentsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
}

/** Agent backend options. */
const BACKEND_OPTIONS: { value: AgentBackend; label: string }[] = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini" },
  { value: "kimi", label: "Kimi" },
  { value: "opencode", label: "OpenCode" },
];

/** Settings key for each CLI's enabled-models field (comma-separated). */
type ModelCsvKey = "claudeModel" | "codexModel" | "geminiModel" | "kimiModel" | "opencodeModel";

/** Map from backend name to its comma-separated enabled-models settings key. */
const BACKEND_CSV_KEY: Record<AgentBackend, ModelCsvKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

/** Parses a comma-separated model string into an array. */
function parseEnabledModels(csv: string): string[] {
  return csv.split(",").map((s) => s.trim()).filter(Boolean);
}

/** Returns enabled model options for a given backend, filtered by settings. */
function getModelOptionsForBackend(
  backend: AgentBackend,
  settings: AppSettings,
): { value: string; label: string }[] {
  const csvKey = BACKEND_CSV_KEY[backend];
  const enabled = new Set(parseEnabledModels(settings[csvKey]));
  const allOptions = CLI_MODEL_OPTIONS[backend] ?? [];
  return allOptions.filter((opt) => enabled.has(opt.value));
}

/** Finds the display label for a backend value. */
function backendLabel(backend: AgentBackend): string {
  return BACKEND_OPTIONS.find((o) => o.value === backend)?.label ?? backend;
}

/** Finds the display label for a model value within a backend. */
function modelLabel(backend: AgentBackend, model: string): string {
  const allOptions = CLI_MODEL_OPTIONS[backend] ?? [];
  return allOptions.find((o) => o.value === model)?.label ?? model;
}

// ---------------------------------------------------------------------------
// CascadingSelect — two-level hover dropdown (backend → models)
// ---------------------------------------------------------------------------

interface CascadingSelectProps {
  backend: AgentBackend;
  model: string;
  settings: AppSettings;
  optional?: boolean;
  onChange: (backend: AgentBackend | null, model: string | null) => void;
}

function CascadingSelect({
  backend,
  model,
  settings,
  optional,
  onChange,
}: CascadingSelectProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [hoveredBackend, setHoveredBackend] = useState<AgentBackend | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent): void {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  // Close on Escape
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    },
    [],
  );
  useEffect(() => {
    if (!open) return;
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, handleKeyDown]);

  const isSkipped = !backend;
  const displayText = isSkipped
    ? "Skip"
    : `${backendLabel(backend)} · ${modelLabel(backend, model)}`;

  return (
    <div ref={containerRef} className="relative">
      {/* Trigger button */}
      <button
        type="button"
        onClick={() => {
          setOpen((prev) => !prev);
          setHoveredBackend(null);
        }}
        className={`flex w-full items-center justify-between rounded border px-3 py-2 text-sm transition-colors ${
          isSkipped
            ? "border-[#2e2e48] bg-[#1a1a24] text-[#6b6b80]"
            : "border-[#2e2e48] bg-[#1a1a24] text-[#e4e4ed]"
        } hover:border-[#6366f1] focus:border-[#6366f1] focus:outline-none`}
      >
        <span className="truncate">{displayText}</span>
        <svg
          className={`ml-2 h-3.5 w-3.5 shrink-0 text-[#6b6b80] transition-transform ${open ? "rotate-180" : ""}`}
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M3 4.5L6 7.5L9 4.5" />
        </svg>
      </button>

      {/* Dropdown popover */}
      {open && (
        <div className="absolute left-0 z-50 mt-1 flex">
          {/* Backend list */}
          <div className="min-w-[140px] rounded-l border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
            {optional && (
              <button
                type="button"
                onClick={() => {
                  onChange(null, null);
                  setOpen(false);
                }}
                onMouseEnter={() => setHoveredBackend(null)}
                className={`flex w-full items-center px-3 py-1.5 text-sm transition-colors ${
                  isSkipped
                    ? "bg-[#6366f1]/15 text-[#e4e4ed]"
                    : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                }`}
              >
                Skip
              </button>
            )}
            {BACKEND_OPTIONS.map((opt) => {
              const isActive = opt.value === hoveredBackend;
              const isCurrent = opt.value === backend;
              return (
                <button
                  key={opt.value}
                  type="button"
                  onMouseEnter={() => setHoveredBackend(opt.value)}
                  onClick={() => setHoveredBackend(opt.value)}
                  className={`flex w-full items-center justify-between px-3 py-1.5 text-sm transition-colors ${
                    isActive
                      ? "bg-[#24243a] text-[#e4e4ed]"
                      : isCurrent
                        ? "text-[#e4e4ed]"
                        : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                  }`}
                >
                  <span>{opt.label}</span>
                  <svg
                    className="ml-2 h-3 w-3 shrink-0 text-[#6b6b80]"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M4.5 3L7.5 6L4.5 9" />
                  </svg>
                </button>
              );
            })}
          </div>

          {/* Model submenu */}
          {hoveredBackend && (
            <div className="min-w-[150px] rounded-r border border-l-0 border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
              {(() => {
                const models = getModelOptionsForBackend(hoveredBackend, settings);
                if (models.length === 0) {
                  return (
                    <span className="block px-3 py-1.5 text-xs text-[#6b6b80]">
                      No models enabled
                    </span>
                  );
                }
                return models.map((m) => {
                  const isSelected = hoveredBackend === backend && m.value === model;
                  return (
                    <button
                      key={m.value}
                      type="button"
                      onClick={() => {
                        onChange(hoveredBackend, m.value);
                        setOpen(false);
                      }}
                      className={`flex w-full items-center gap-2 px-3 py-1.5 text-sm transition-colors ${
                        isSelected
                          ? "bg-[#6366f1]/15 text-[#e4e4ed]"
                          : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                      }`}
                    >
                      {isSelected && (
                        <svg
                          className="h-3 w-3 shrink-0 text-[#6366f1]"
                          viewBox="0 0 12 12"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        >
                          <path d="M2.5 6L5 8.5L9.5 3.5" />
                        </svg>
                      )}
                      <span>{m.label}</span>
                    </button>
                  );
                });
              })()}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Stage definitions
// ---------------------------------------------------------------------------

interface StageConfig {
  label: string;
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
  optional: boolean;
}

const STAGES: StageConfig[] = [
  { label: "Prompt Enhancer", backendKey: "promptEnhancerAgent", modelKey: "promptEnhancerModel", optional: false },
  { label: "Planner", backendKey: "plannerAgent", modelKey: "plannerModel", optional: true },
  { label: "Plan Auditor", backendKey: "planAuditorAgent", modelKey: "planAuditorModel", optional: true },
  { label: "Coder", backendKey: "generatorAgent", modelKey: "generatorModel", optional: false },
  { label: "Code Reviewer", backendKey: "reviewerAgent", modelKey: "reviewerModel", optional: false },
  { label: "Code Fixer", backendKey: "fixerAgent", modelKey: "fixerModel", optional: false },
  { label: "Judge", backendKey: "finalJudgeAgent", modelKey: "finalJudgeModel", optional: false },
  { label: "Executive Summary", backendKey: "executiveSummaryAgent", modelKey: "executiveSummaryModel", optional: false },
];

// ---------------------------------------------------------------------------
// Main view
// ---------------------------------------------------------------------------

/** Inline view for configuring agent role assignments and pipeline parameters. */
export function AgentsView({ settings, onSave }: AgentsViewProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  function update(patch: Partial<AppSettings>): void {
    setDraft((prev) => ({ ...prev, ...patch }));
  }

  function handleSave(): void {
    onSave(draft);
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Agents</h1>
          <p className="text-sm text-[#9898b0]">
            Configure which CLI backend and model handles each pipeline role.
          </p>

          {/* Agent cards — 2-column grid */}
          <div className="grid gap-4 grid-cols-1 sm:grid-cols-2">
            {STAGES.map((stage) => {
              const currentBackend = draft[stage.backendKey] as AgentBackend | null;
              const currentModel = draft[stage.modelKey] as string | null;

              return (
                <div
                  key={stage.label}
                  className="rounded-lg border border-[#2e2e48] bg-[#1a1a2e] p-4 flex flex-col gap-2"
                >
                  <span className="text-xs font-medium text-[#9898b0]">
                    {stage.label}
                    {stage.optional && (
                      <span className="ml-1 text-[#6b6b80]">(optional)</span>
                    )}
                  </span>
                  <CascadingSelect
                    backend={currentBackend as AgentBackend}
                    model={currentModel ?? ""}
                    settings={draft}
                    optional={stage.optional}
                    onChange={(newBackend, newModel) => {
                      update({
                        [stage.backendKey]: newBackend,
                        [stage.modelKey]: newModel,
                      });
                    }}
                  />
                </div>
              );
            })}
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
