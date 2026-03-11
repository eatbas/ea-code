import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, CliHealth } from "../types";
import { TextInput, AgentSelect, OptionalAgentSelect, HealthDot } from "./shared/FormInputs";

interface SettingsPanelProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  health?: CliHealth;
  onCheckHealth: () => void;
  onClose: () => void;
}

/** Settings modal overlay for configuring CLI paths, agent roles, and pipeline parameters. */
export function SettingsPanel({
  settings,
  onSave,
  health,
  onCheckHealth,
  onClose,
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
    onClose();
  }

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/60"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-[520px] max-h-[80vh] overflow-y-auto rounded-xl border border-[#2e2e48] bg-[#1a1a24] shadow-2xl">
        {/* Modal header */}
        <div className="flex items-center justify-between border-b border-[#2e2e48] px-5 py-4">
          <h2 className="text-sm font-bold text-[#e4e4ed]">Settings</h2>
          <button
            onClick={onClose}
            className="rounded p-1 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Modal body */}
        <div className="flex flex-col gap-4 p-5">
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
            <div className="flex items-center gap-2">
              <div className="flex-1">
                <TextInput label="Kimi" value={draft.kimiPath} onChange={(v) => update({ kimiPath: v })} />
              </div>
              {health && <HealthDot available={health.kimi.available} error={health.kimi.error} />}
            </div>
            <div className="flex items-center gap-2">
              <div className="flex-1">
                <TextInput label="OpenCode" value={draft.opencodePath} onChange={(v) => update({ opencodePath: v })} />
              </div>
              {health && <HealthDot available={health.opencode.available} error={health.opencode.error} />}
            </div>
          </fieldset>

          {/* Agent role mapping */}
          <fieldset className="flex flex-col gap-2">
            <legend className="text-xs font-medium text-[#9898b0] mb-1">Agent Roles</legend>
            <span className="text-[10px] text-[#6b6b82]">Roles marked as minimum must be set before prompts can be sent.</span>
            <AgentSelect label="Prompt Enhancer (minimum)" value={draft.promptEnhancerAgent} onChange={(v) => update({ promptEnhancerAgent: v })} />
            <OptionalAgentSelect label="Planner" value={draft.plannerAgent} onChange={(v) => update({ plannerAgent: v })} />
            <OptionalAgentSelect label="Plan Auditor" value={draft.planAuditorAgent} onChange={(v) => update({ planAuditorAgent: v })} />
            <AgentSelect label="Coder (minimum)" value={draft.coderAgent} onChange={(v) => update({ coderAgent: v })} />
            <AgentSelect label="Code Reviewer / Auditor (minimum)" value={draft.codeReviewerAgent} onChange={(v) => update({ codeReviewerAgent: v })} />
            <AgentSelect label="Code Fixer (minimum)" value={draft.codeFixerAgent} onChange={(v) => update({ codeFixerAgent: v })} />
            <AgentSelect label="Judge (minimum)" value={draft.finalJudgeAgent} onChange={(v) => update({ finalJudgeAgent: v })} />
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
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Agent Retry Count</span>
              <input
                type="number"
                min={0}
                max={5}
                value={draft.agentRetryCount}
                onChange={(e) => update({ agentRetryCount: Math.max(0, Math.min(5, Number(e.target.value))) })}
                className="w-20 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
              <span className="text-[10px] text-[#6b6b82]">Retries per agent call on failure</span>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Agent Timeout (ms)</span>
              <input
                type="number"
                min={0}
                step={1000}
                value={draft.agentTimeoutMs}
                onChange={(e) => update({ agentTimeoutMs: Math.max(0, Number(e.target.value)) })}
                className="w-28 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
              <span className="text-[10px] text-[#6b6b82]">Per-agent timeout (0 = no timeout)</span>
            </label>
            <label className="flex flex-col gap-1">
              <span className="text-xs font-medium text-[#9898b0]">Data Retention (days)</span>
              <input
                type="number"
                min={0}
                max={3650}
                value={draft.retentionDays}
                onChange={(e) => update({ retentionDays: Math.max(0, Math.min(3650, Number(e.target.value))) })}
                className="w-20 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
              />
              <span className="text-[10px] text-[#6b6b82]">Completed runs older than this are deleted on startup (0 = keep forever)</span>
            </label>
          </fieldset>

          {/* Plan Gate */}
          <fieldset className="flex flex-col gap-2">
            <legend className="text-xs font-medium text-[#9898b0] mb-1">Plan Gate</legend>
            <label className="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                checked={draft.requirePlanApproval}
                onChange={(e) => update({ requirePlanApproval: e.target.checked })}
                className="rounded border-[#2e2e48] accent-[#6366f1]"
              />
              <span className="text-xs text-[#9898b0]">Require plan approval</span>
            </label>
            {draft.requirePlanApproval && (
              <>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-medium text-[#9898b0]">Auto-Approve Timeout (sec)</span>
                  <input
                    type="number"
                    min={0}
                    max={600}
                    value={draft.planAutoApproveTimeoutSec}
                    onChange={(e) => update({ planAutoApproveTimeoutSec: Math.max(0, Math.min(600, Number(e.target.value))) })}
                    className="w-20 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
                  />
                  <span className="text-[10px] text-[#6b6b82]">Seconds before auto-approve (0 = wait)</span>
                </label>
                <label className="flex flex-col gap-1">
                  <span className="text-xs font-medium text-[#9898b0]">Max Plan Revisions</span>
                  <input
                    type="number"
                    min={0}
                    max={10}
                    value={draft.maxPlanRevisions}
                    onChange={(e) => update({ maxPlanRevisions: Math.max(0, Math.min(10, Number(e.target.value))) })}
                    className="w-20 rounded border border-[#2e2e48] bg-[#0f0f14] px-3 py-1.5 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
                  />
                  <span className="text-[10px] text-[#6b6b82]">Max revision rounds</span>
                </label>
              </>
            )}
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
      </div>
    </div>
  );
}
