import type { ReactNode } from "react";
import { useCallback, useEffect } from "react";
import type { ApiHealth, AppSettings, ProviderInfo, ApiCliVersionInfo } from "../../types";
import { modelOptionsFromProvider, providerDisplayName } from "../shared/constants";
import { useToast } from "../shared/Toast";
import { CliCard } from "./CliCard";

/** Legacy per-CLI model CSV settings keys. */
const LEGACY_MODEL_KEY: Record<string, keyof AppSettings> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

function parseEnabledModels(csv: string): Set<string> {
  return new Set(csv.split(",").map((s) => s.trim()).filter(Boolean));
}

function serialiseEnabledModels(models: Set<string>): string {
  return Array.from(models).join(",");
}

interface CliSetupViewProps {
  settings: AppSettings;
  apiHealth: ApiHealth | null;
  providers: ProviderInfo[];
  apiVersions: ApiCliVersionInfo[];
  versionsLoading: boolean;
  updating: string | null;
  onFetchVersions: () => void;
  onRefreshProviders: () => void;
  onUpdateCli: (provider: string) => Promise<void>;
  onSave: (settings: AppSettings) => void;
}

export function CliSetupView({
  settings,
  apiHealth,
  providers,
  apiVersions,
  versionsLoading,
  updating,
  onFetchVersions,
  onRefreshProviders,
  onUpdateCli,
  onSave,
}: CliSetupViewProps): ReactNode {
  const toast = useToast();
  const actionsDisabled = versionsLoading || updating !== null;

  const refreshAll = useCallback((showSuccessToast: boolean): void => {
    onRefreshProviders();
    onFetchVersions();
    if (showSuccessToast) {
      toast.success("CLI version check started.");
    }
  }, [onFetchVersions, onRefreshProviders, toast]);

  useEffect(() => {
    refreshAll(false);
  }, [refreshAll]);

  function getEnabledModels(providerName: string): Set<string> {
    // Check providerModels first, then legacy fields.
    const dynamic = settings.providerModels?.[providerName];
    if (dynamic !== undefined) return parseEnabledModels(dynamic);
    const legacyKey = LEGACY_MODEL_KEY[providerName];
    if (legacyKey) return parseEnabledModels(settings[legacyKey] as string);
    return new Set();
  }

  function handleToggleModel(providerName: string, model: string): void {
    if (actionsDisabled) return;
    const provider = providers.find((p) => p.name === providerName);
    if (provider && !provider.available) return;

    const current = getEnabledModels(providerName);
    if (current.has(model)) {
      current.delete(model);
    } else {
      current.add(model);
    }

    const csv = serialiseEnabledModels(current);
    const legacyKey = LEGACY_MODEL_KEY[providerName];

    let updated: AppSettings;
    if (legacyKey) {
      // Write to both legacy field and providerModels for consistency.
      updated = {
        ...settings,
        [legacyKey]: csv,
        providerModels: { ...settings.providerModels, [providerName]: csv },
      };
    } else {
      updated = {
        ...settings,
        providerModels: { ...settings.providerModels, [providerName]: csv },
      };
    }
    onSave(updated);
  }

  function handleToggleAll(providerName: string, selectAll: boolean): void {
    if (actionsDisabled) return;
    const provider = providers.find((p) => p.name === providerName);
    if (provider && !provider.available) return;

    const allValues = modelOptionsFromProvider(provider).map((opt) => opt.value);
    const updated: Set<string> = selectAll ? new Set(allValues) : new Set();
    const csv = serialiseEnabledModels(updated);
    const legacyKey = LEGACY_MODEL_KEY[providerName];

    let next: AppSettings;
    if (legacyKey) {
      next = {
        ...settings,
        [legacyKey]: csv,
        providerModels: { ...settings.providerModels, [providerName]: csv },
      };
    } else {
      next = {
        ...settings,
        providerModels: { ...settings.providerModels, [providerName]: csv },
      };
    }
    onSave(next);
  }

  async function handleUpdateCli(providerName: string): Promise<void> {
    if (actionsDisabled) return;
    await onUpdateCli(providerName);
    toast.success(`${providerDisplayName(providerName)} CLI update completed.`);
  }

  function versionForProvider(providerName: string): ApiCliVersionInfo | undefined {
    return apiVersions.find((v) => v.provider === providerName);
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
              onClick={() => refreshAll(true)}
              disabled={actionsDisabled}
              className="rounded-md border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm font-medium text-[#9898b0] transition-colors hover:bg-[#2e2e48] hover:text-[#e4e4ed] disabled:cursor-not-allowed disabled:opacity-50"
            >
              {versionsLoading ? "Checking..." : updating ? "Updating..." : "Refresh"}
            </button>
          </div>
          {/* Hive-API status */}
          <div className="flex items-center gap-3 rounded-lg border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
            <span
              className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${
                apiHealth?.connected ? "bg-[#22c55e]" : "bg-[#ef4444]"
              }`}
            />
            <div className="flex flex-1 items-center justify-between">
              <div>
                <span className="text-sm font-medium text-[#e4e4ed]">Hive API</span>
                <span className="ml-2 text-xs text-[#6b6b80]">
                  {apiHealth?.connected
                    ? `Connected${apiHealth.droneCount != null ? ` · ${apiHealth.droneCount} drone${apiHealth.droneCount !== 1 ? "s" : ""}` : ""}`
                    : apiHealth?.error ?? "Not connected"}
                </span>
              </div>
              {apiHealth?.connected && apiHealth.url && (
                <span className="text-xs font-mono text-[#6b6b80]">{apiHealth.url}</span>
              )}
            </div>
          </div>

          {providers.length > 0 && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {providers.map((p) => (
                <CliCard
                  key={p.name}
                  provider={p}
                  version={versionForProvider(p.name)}
                  loading={versionsLoading || updating === p.name}
                  updating={updating === p.name}
                  actionsDisabled={actionsDisabled}
                  enabledModels={getEnabledModels(p.name)}
                  onToggleModel={(model) => handleToggleModel(p.name, model)}
                  onToggleAll={(selectAll) => handleToggleAll(p.name, selectAll)}
                  onUpdate={() => void handleUpdateCli(p.name)}
                />
              ))}
            </div>
          )}
          {providers.length === 0 && !versionsLoading && (
            <p className="text-sm text-[#6b6b80]">
              No providers detected. Ensure the hive-api sidecar is running.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
