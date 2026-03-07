import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { AppSettings, CliHealth } from "../types";

interface CliSetupViewProps {
  settings: AppSettings;
  onSave: (s: AppSettings) => void;
  health?: CliHealth;
  onCheckHealth: () => void;
}

/** Reusable text input row. */
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
        className="rounded border border-[#2e2e48] bg-[#1a1a24] px-3 py-2 text-sm text-[#e4e4ed] focus:border-[#6366f1] focus:outline-none"
      />
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

/** Inline view for configuring CLI paths and checking health. */
export function CliSetupView({ settings, onSave, health, onCheckHealth }: CliSetupViewProps): ReactNode {
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
        <div className="mx-auto max-w-lg flex flex-col gap-6">
          <h1 className="text-xl font-bold text-[#e4e4ed]">CLI Setup</h1>
          <p className="text-sm text-[#9898b0]">
            Set the paths to each agent CLI and verify they are available.
          </p>

          {/* CLI paths */}
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
          </div>

          {/* Action buttons */}
          <div className="flex gap-2">
            <button
              onClick={handleSave}
              className="rounded bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] hover:bg-white transition-colors"
            >
              Save
            </button>
            <button
              onClick={onCheckHealth}
              className="rounded border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed] transition-colors"
            >
              Check Health
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
