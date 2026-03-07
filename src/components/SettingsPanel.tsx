import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, AgentBackend, CliHealth } from "../types";

interface SettingsPanelProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  health?: CliHealth;
  onCheckHealth: () => void;
}

/** Agent backend options for dropdown selects. */
const BACKEND_OPTIONS: { value: AgentBackend; label: string }[] = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini" },
];

/** Reusable text input row for the settings form. */
function TextInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}): ReactNode {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#9898b0]">{label}</span>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
      />
    </label>
  );
}

/** Reusable select dropdown row for agent role mapping. */
function AgentSelect({
  label,
  value,
  onChange,
}: {
  label: string;
  value: AgentBackend;
  onChange: (v: AgentBackend) => void;
}): ReactNode {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#9898b0]">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as AgentBackend)}
        className="rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
      >
        {BACKEND_OPTIONS.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Health status indicator dot. */
function HealthDot({ available, error }: { available: boolean; error?: string }): ReactNode {
  return (
    <span
      title={error ?? (available ? "Available" : "Not found")}
      className={`inline-block h-2.5 w-2.5 rounded-full ${
        available ? "bg-[#22c55e]" : "bg-[#ef4444]"
      }`}
    />
  );
}

/** Settings form for configuring CLI paths, agent roles, and pipeline parameters. */
export function SettingsPanel({
  settings,
  onSave,
  health,
  onCheckHealth,
}: SettingsPanelProps): ReactNode {
  const [draft, setDraft] = useState<AppSettings>(settings);

  // Sync draft when settings change externally
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
    <div className="flex flex-col gap-4 p-4 overflow-y-auto">
      <h2 className="text-sm font-bold text-[#e4e4ed]">Settings</h2>

      {/* CLI paths */}
      <fieldset className="flex flex-col gap-2">
        <legend className="text-xs font-medium text-[#9898b0] mb-1">CLI Paths</legend>
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <TextInput label="Claude" value={draft.claudePath} onChange={(v) => update({ claudePath: v })} />
          </div>
          {health && <HealthDot available={health.claude.available} error={health.claude.error} />}
        </div>
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <TextInput label="Codex" value={draft.codexPath} onChange={(v) => update({ codexPath: v })} />
          </div>
          {health && <HealthDot available={health.codex.available} error={health.codex.error} />}
        </div>
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <TextInput label="Gemini" value={draft.geminiPath} onChange={(v) => update({ geminiPath: v })} />
          </div>
          {health && <HealthDot available={health.gemini.available} error={health.gemini.error} />}
        </div>
      </fieldset>

      {/* Agent role mapping */}
      <fieldset className="flex flex-col gap-2">
        <legend className="text-xs font-medium text-[#9898b0] mb-1">Agent Roles</legend>
        <AgentSelect label="Generator" value={draft.generatorAgent} onChange={(v) => update({ generatorAgent: v })} />
        <AgentSelect label="Reviewer" value={draft.reviewerAgent} onChange={(v) => update({ reviewerAgent: v })} />
        <AgentSelect label="Fixer" value={draft.fixerAgent} onChange={(v) => update({ fixerAgent: v })} />
        <AgentSelect label="Validator" value={draft.validatorAgent} onChange={(v) => update({ validatorAgent: v })} />
        <AgentSelect label="Final Judge" value={draft.finalJudgeAgent} onChange={(v) => update({ finalJudgeAgent: v })} />
      </fieldset>

      {/* Pipeline parameters */}
      <fieldset className="flex flex-col gap-2">
        <legend className="text-xs font-medium text-[#9898b0] mb-1">Pipeline</legend>
        <label className="flex flex-col gap-1">
          <span className="text-xs font-medium text-[#9898b0]">Max Iterations</span>
          <input
            type="number"
            min={1}
            max={10}
            value={draft.maxIterations}
            onChange={(e) => update({ maxIterations: Math.max(1, Math.min(10, Number(e.target.value))) })}
            className="w-20 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
          />
        </label>
        <label className="flex items-center gap-2 cursor-pointer">
          <input
            type="checkbox"
            checked={draft.requireGit}
            onChange={(e) => update({ requireGit: e.target.checked })}
            className="rounded border-[#2e2e48] accent-[#6366f1]"
          />
          <span className="text-xs text-[#9898b0]">Require Git repository</span>
        </label>
      </fieldset>

      {/* Action buttons */}
      <div className="flex gap-2">
        <button
          onClick={handleSave}
          className="rounded bg-[#6366f1] px-4 py-2 text-sm font-medium text-white hover:bg-[#818cf8] transition-colors"
        >
          Save
        </button>
        <button
          onClick={onCheckHealth}
          className="rounded border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed] transition-colors"
        >
          Check CLI Health
        </button>
      </div>
    </div>
  );
}
