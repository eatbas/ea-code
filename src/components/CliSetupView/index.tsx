import type { ReactNode } from "react";
import { useEffect } from "react";
import type { AppSettings, CliVersionInfo, AllCliVersions } from "../../types";
import { CLI_MODEL_OPTIONS } from "../../types";
import { CliCard } from "./CliCard";

type ModelSettingsKey =
  | "claudeModel"
  | "codexModel"
  | "geminiModel"
  | "kimiModel"
  | "opencodeModel";

const MODEL_KEY_MAP: Record<string, ModelSettingsKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

interface CliSetupViewProps {
  settings: AppSettings;
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  onFetchVersions: (settings: AppSettings) => void;
  onUpdateCli: (cliName: string, settings: AppSettings) => void;
  onSave: (settings: AppSettings) => void;
}

function parseEnabledModels(csv: string): Set<string> {
  return new Set(csv.split(",").map((s) => s.trim()).filter(Boolean));
}

function serialiseEnabledModels(models: Set<string>): string {
  return Array.from(models).join(",");
}

export function CliSetupView({
  settings,
  versions,
  loading,
  updating,
  error,
  onFetchVersions,
  onUpdateCli,
  onSave,
}: CliSetupViewProps): ReactNode {
  useEffect(() => {
    onFetchVersions(settings);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const cliEntries: CliVersionInfo[] = versions
    ? [
        versions.claude,
        versions.codex,
        versions.gemini,
        versions.kimi,
        versions.opencode,
        ...(versions.gitBash ? [versions.gitBash] : []),
      ]
    : [];

  function getEnabledModels(cliName: string): Set<string> {
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return new Set();
    return parseEnabledModels(settings[key]);
  }

  function handleToggleModel(cliName: string, model: string): void {
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return;
    const cliInfo = versions?.[cliName as keyof AllCliVersions];
    if (cliInfo && !cliInfo.available) return;
    const current = parseEnabledModels(settings[key]);
    if (current.has(model)) {
      if (current.size <= 1) return;
      current.delete(model);
    } else {
      current.add(model);
    }
    const updated = { ...settings, [key]: serialiseEnabledModels(current) };
    onSave(updated);
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-bold text-[#e4e4ed]">CLI Setup</h1>
              <p className="mt-1 text-sm text-[#9898b0]">
                Manage your agent CLI tools and keep them up to date.
              </p>
            </div>
            <button
              onClick={() => onFetchVersions(settings)}
              disabled={loading}
              className="rounded-md border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] transition-colors hover:bg-[#2e2e48] hover:text-[#e4e4ed] disabled:cursor-not-allowed disabled:opacity-50"
            >
              {loading ? "Checking…" : "Refresh"}
            </button>
          </div>
          {error && (
            <div className="rounded-md border border-[#ef4444]/30 bg-[#ef4444]/10 px-4 py-3 text-sm text-[#ef4444]">
              {error}
            </div>
          )}
          {loading && !versions && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {[0, 1, 2, 3, 4, 5].map((i) => (
                <div
                  key={i}
                  className="h-48 animate-pulse rounded-lg border border-[#2e2e48] bg-[#1a1a24]"
                />
              ))}
            </div>
          )}
          {cliEntries.length > 0 && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {cliEntries.map((info) => (
                <CliCard
                  key={info.cliName}
                  info={info}
                  updating={updating === info.cliName}
                  enabledModels={getEnabledModels(info.cliName)}
                  modelOptions={CLI_MODEL_OPTIONS[info.cliName] ?? []}
                  onToggleModel={(model) => handleToggleModel(info.cliName, model)}
                  onUpdate={() => onUpdateCli(info.cliName, settings)}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
