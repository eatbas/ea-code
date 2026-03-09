import type { ReactNode } from "react";
import { useCallback, useEffect } from "react";
import type { AppSettings, CliVersionInfo, AllCliVersions } from "../../types";
import { CLI_MODEL_OPTIONS } from "../../types";
import { sanitiseAgentAssignmentsForEnabledModels } from "../../utils/agentSettings";
import { useToast } from "../shared/Toast";
import { CliCard } from "./CliCard";

type ModelSettingsKey =
  | "claudeModel"
  | "codexModel"
  | "geminiModel"
  | "kimiModel"
  | "opencodeModel";

interface CliActionResult {
  success: boolean;
  message?: string;
}

const MODEL_KEY_MAP: Record<string, ModelSettingsKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

const FALLBACK_CLI_ENTRIES: CliVersionInfo[] = [
  {
    name: "Claude CLI",
    cliName: "claude",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
  {
    name: "Codex CLI",
    cliName: "codex",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
  {
    name: "Gemini CLI",
    cliName: "gemini",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
  {
    name: "Kimi CLI",
    cliName: "kimi",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
  {
    name: "OpenCode CLI",
    cliName: "opencode",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
  {
    name: "Git Bash CLI",
    cliName: "gitBash",
    upToDate: false,
    updateCommand: "",
    available: true,
  },
];

interface CliSetupViewProps {
  settings: AppSettings;
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  onFetchVersions: (settings: AppSettings) => Promise<CliActionResult>;
  onUpdateCli: (cliName: string, settings: AppSettings) => Promise<CliActionResult>;
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
  onFetchVersions,
  onUpdateCli,
  onSave,
}: CliSetupViewProps): ReactNode {
  const toast = useToast();
  const actionsDisabled = loading || updating !== null;

  const cliEntries: CliVersionInfo[] = versions
    ? [
        versions.claude,
        versions.codex,
        versions.gemini,
        versions.kimi,
        versions.opencode,
        ...(versions.gitBash ? [versions.gitBash] : []),
      ]
    : FALLBACK_CLI_ENTRIES;

  const refreshVersions = useCallback(async (showSuccessToast: boolean): Promise<void> => {
    const result = await onFetchVersions(settings);
    if (!result.success) {
      return;
    }
    if (showSuccessToast) {
      toast.success("CLI versions refreshed.");
    }
  }, [onFetchVersions, settings, toast]);

  useEffect(() => {
    void refreshVersions(false);
  }, [refreshVersions]);

  function getEnabledModels(cliName: string): Set<string> {
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return new Set();
    return parseEnabledModels(settings[key]);
  }

  function handleToggleModel(cliName: string, model: string): void {
    if (actionsDisabled) return;
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return;
    const cliInfo = versions?.[cliName as keyof AllCliVersions];
    if (cliInfo && !cliInfo.available) return;
    const current = parseEnabledModels(settings[key]);
    if (current.has(model)) {
      current.delete(model);
    } else {
      current.add(model);
    }
    const updated = { ...settings, [key]: serialiseEnabledModels(current) };
    onSave(sanitiseAgentAssignmentsForEnabledModels(updated));
  }

  async function handleUpdateCli(cliName: string, cliDisplayName: string): Promise<void> {
    if (actionsDisabled) return;
    const result = await onUpdateCli(cliName, settings);
    if (result.success) {
      toast.success(result.message?.trim() || `${cliDisplayName} update completed.`);
    }
  }

  return (
    <div className="relative flex h-full flex-col bg-[#0f0f14]">
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
              type="button"
              onClick={() => void refreshVersions(true)}
              disabled={actionsDisabled}
              className="rounded-md border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] transition-colors hover:bg-[#2e2e48] hover:text-[#e4e4ed] disabled:cursor-not-allowed disabled:opacity-50"
            >
              {loading ? "Checking..." : updating ? "Updating..." : "Refresh"}
            </button>
          </div>
          {cliEntries.length > 0 && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {cliEntries.map((info) => (
                <CliCard
                  key={info.cliName}
                  info={info}
                  loading={loading || updating === info.cliName}
                  updating={updating === info.cliName}
                  actionsDisabled={actionsDisabled}
                  enabledModels={getEnabledModels(info.cliName)}
                  modelOptions={CLI_MODEL_OPTIONS[info.cliName] ?? []}
                  onToggleModel={(model) => handleToggleModel(info.cliName, model)}
                  onUpdate={() => void handleUpdateCli(info.cliName, info.name)}
                />
              ))}
            </div>
          )}
        </div>
      </div>

    </div>
  );
}
