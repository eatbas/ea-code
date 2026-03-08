import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, CliHealth } from "../types";
import { TextInput, AgentSelect, OptionalAgentSelect, HealthDot } from "./shared/FormInputs";

interface SettingsViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  health?: CliHealth;
  onCheckHealth: () => void;
}

/** Inline settings view combining Agents and CLI Setup sections. */
export function SettingsView({ settings, onSave, health, onCheckHealth }: SettingsViewProps): ReactNode {
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
        <div className="mx-auto max-w-lg flex flex-col gap-8">
          <h1 className="text-xl font-bold text-[#e4e4ed]">Settings</h1>

          {/* CLI Setup section */}
          <section className="flex flex-col gap-3">
            <h2 className="text-sm font-medium text-[#e4e4ed]">CLI Setup</h2>
            <p className="text-xs text-[#9898b0]">
              Set the paths to each agent CLI and verify availability.
            </p>
            <div className="flex flex-col gap-3">
              <div className="flex items-end gap-3">
                <div className="flex-1">
                  <TextInput label="Claude" value={draft.claudePath} onChange={(v) => update({ claudePath: v })} />
                </div>
                {health && <HealthDot available={health.claude.available} error={health.claude.error} />}
              </div>
              <div className="flex items-end gap-3">
                <div className="flex-1">
                  <TextInput label="Codex" value={draft.codexPath} onChange={(v) => update({ codexPath: v })} />
                </div>
                {health && <HealthDot available={health.codex.available} error={health.codex.error} />}
              </div>
              <div className="flex items-end gap-3">
                <div className="flex-1">
                  <TextInput label="Gemini" value={draft.geminiPath} onChange={(v) => update({ geminiPath: v })} />
                </div>
                {health && <HealthDot available={health.gemini.available} error={health.gemini.error} />}
              </div>
              <div className="flex items-end gap-3">
                <div className="flex-1">
                  <TextInput label="Kimi" value={draft.kimiPath} onChange={(v) => update({ kimiPath: v })} />
                </div>
                {health && <HealthDot available={health.kimi.available} error={health.kimi.error} />}
              </div>
              <div className="flex items-end gap-3">
                <div className="flex-1">
                  <TextInput label="OpenCode" value={draft.opencodePath} onChange={(v) => update({ opencodePath: v })} />
                </div>
                {health && <HealthDot available={health.opencode.available} error={health.opencode.error} />}
              </div>
            </div>
            <button
              onClick={onCheckHealth}
              className="self-start rounded border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed] transition-colors"
            >
              Check Health
            </button>
          </section>

          {/* Agents section */}
          <section className="flex flex-col gap-3 border-t border-[#2e2e48] pt-6">
            <h2 className="text-sm font-medium text-[#e4e4ed]">Agents</h2>
            <p className="text-xs text-[#9898b0]">
              Configure which CLI backend handles each pipeline role.
            </p>
            <div className="flex flex-col gap-3">
              <AgentSelect label="Prompt Enhancer" value={draft.promptEnhancerAgent} onChange={(v) => update({ promptEnhancerAgent: v })} />
              <OptionalAgentSelect label="Planner" value={draft.plannerAgent} onChange={(v) => update({ plannerAgent: v })} />
              <OptionalAgentSelect label="Plan Auditor" value={draft.planAuditorAgent} onChange={(v) => update({ planAuditorAgent: v })} />
              <AgentSelect label="Coder" value={draft.generatorAgent} onChange={(v) => update({ generatorAgent: v })} />
              <AgentSelect label="Code Reviewer / Auditor" value={draft.reviewerAgent} onChange={(v) => update({ reviewerAgent: v })} />
              <AgentSelect label="Code Fixer" value={draft.fixerAgent} onChange={(v) => update({ fixerAgent: v })} />
              <AgentSelect label="Judge" value={draft.finalJudgeAgent} onChange={(v) => update({ finalJudgeAgent: v })} />
            </div>
          </section>

          {/* Pipeline section */}
          <section className="flex flex-col gap-3 border-t border-[#2e2e48] pt-6">
            <h2 className="text-sm font-medium text-[#e4e4ed]">Pipeline</h2>
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
          </section>

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
