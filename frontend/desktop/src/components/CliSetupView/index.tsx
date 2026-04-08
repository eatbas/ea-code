import type { ReactNode } from "react";
import { useCallback, useEffect, useRef } from "react";
import type { ApiHealth, AppSettings, ProviderInfo, ApiCliVersionInfo } from "../../types";
import {
  modelOptionsFromProvider,
  providerDisplayName,
  sortProvidersByDisplayName,
} from "../shared/constants";
import { useToast } from "../shared/Toast";
import {
  getEnabledModels,
  serialiseEnabledModels,
  applyModelCsv,
} from "../../utils/modelSettings";
import { CliCard } from "./CliCard";

/** Minimum milliseconds between automatic refreshes on mount. */
const REFRESH_COOLDOWN_MS = 60_000;

/** Build a composite key for the providerThinking map. */
function thinkingKey(provider: string, model: string): string {
  return `${provider}:${model}`;
}

/** Extract the per-model thinking levels for a given provider. */
function thinkingLevelsForProvider(
  providerThinking: Record<string, string>,
  providerName: string,
): Record<string, string> {
  const prefix = `${providerName}:`;
  const levels: Record<string, string> = {};
  for (const [key, value] of Object.entries(providerThinking)) {
    if (key.startsWith(prefix)) {
      levels[key.slice(prefix.length)] = value;
    }
  }
  return levels;
}

interface CliSetupViewProps {
  settings: AppSettings;
  apiHealth: ApiHealth | null;
  sidecarReady: boolean | null;
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
  sidecarReady,
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
  const lastRefreshRef = useRef<number>(0);
  const sortedProviders = sortProvidersByDisplayName(providers);

  const refreshAll = useCallback((showSuccessToast: boolean): void => {
    onRefreshProviders();
    onFetchVersions();
    lastRefreshRef.current = Date.now();
    if (showSuccessToast) {
      toast.success("CLI version check started.");
    }
  }, [onFetchVersions, onRefreshProviders, toast]);

  useEffect(() => {
    if (Date.now() - lastRefreshRef.current >= REFRESH_COOLDOWN_MS) {
      refreshAll(false);
    }
  }, [refreshAll]);

  function handleThinkingChange(providerName: string, model: string, value: string): void {
    const updated: Record<string, string> = { ...settings.providerThinking };
    const key = thinkingKey(providerName, model);
    if (value) {
      updated[key] = value;
    } else {
      delete updated[key];
    }
    onSave({ ...settings, providerThinking: updated });
  }

  function handleSwarmChange(value: string): void {
    onSave({ ...settings, kimiSwarmEnabled: value === "enabled" });
  }

  function handleRalphIterationsChange(value: string): void {
    const parsed = value ? parseInt(value, 10) : 1;
    onSave({ ...settings, kimiMaxRalphIterations: parsed });
  }

  function handleToggleModel(providerName: string, model: string): void {
    if (actionsDisabled) return;
    const provider = providers.find((p) => p.name === providerName);
    if (provider && !provider.available) return;

    const current = getEnabledModels(settings, providerName);
    if (current.has(model)) {
      current.delete(model);
    } else {
      current.add(model);
    }

    const csv = serialiseEnabledModels(current);
    onSave(applyModelCsv(settings, providerName, csv));
  }

  function handleToggleAll(providerName: string, selectAll: boolean): void {
    if (actionsDisabled) return;
    const provider = providers.find((p) => p.name === providerName);
    if (provider && !provider.available) return;

    const allValues = modelOptionsFromProvider(provider).map((opt) => opt.value);
    const next: Set<string> = selectAll ? new Set(allValues) : new Set();
    const csv = serialiseEnabledModels(next);
    onSave(applyModelCsv(settings, providerName, csv));
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
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-xl font-bold text-fg">CLI Setup</h1>
              <p className="mt-1 text-sm text-fg-muted">
                Manage your agent CLI tools and keep them up to date.
              </p>
            </div>
            <button
              type="button"
              onClick={() => refreshAll(true)}
              disabled={actionsDisabled}
              className="rounded-md border border-edge bg-elevated px-4 py-2 text-sm font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg disabled:cursor-not-allowed disabled:opacity-50"
            >
              {versionsLoading ? "Checking..." : updating ? "Updating..." : "Refresh"}
            </button>
          </div>
          {/* Symphony status */}
          <div className="flex items-center gap-3 rounded-lg border border-edge bg-panel px-4 py-3">
            {sidecarReady === null && !apiHealth ? (
              <>
                <span className="inline-block h-2.5 w-2.5 shrink-0 animate-pulse rounded-full bg-fg-faint" />
                <div className="flex flex-1 flex-col gap-1">
                  <span className="text-sm font-medium text-fg">Symphony</span>
                  <div className="h-1.5 w-full overflow-hidden rounded-full bg-edge">
                    <div className="h-full animate-[symphony-loading_1.5s_ease-in-out_infinite] rounded-full bg-fg-muted" />
                  </div>
                  <span className="text-xs text-fg-faint">Starting up...</span>
                </div>
              </>
            ) : (
              <>
                <span
                  className={`inline-block h-2.5 w-2.5 shrink-0 rounded-full ${
                    apiHealth?.connected ? "bg-success" : "bg-danger"
                  }`}
                />
                <div className="flex flex-1 items-center justify-between">
                  <div>
                    <span className="text-sm font-medium text-fg">Symphony</span>
                    <span className="ml-2 text-xs text-fg-faint">
                      {apiHealth?.connected
                        ? `Connected${apiHealth.musicianCount != null ? ` · ${apiHealth.musicianCount} musician${apiHealth.musicianCount !== 1 ? "s" : ""}` : ""}`
                        : apiHealth?.error ?? "Not connected"}
                    </span>
                  </div>
                  {apiHealth?.connected && apiHealth.url && (
                    <span className="text-xs font-mono text-fg-faint">{apiHealth.url}</span>
                  )}
                </div>
              </>
            )}
          </div>

          {providers.length > 0 && (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              {sortedProviders.map((p) => (
                <CliCard
                  key={p.name}
                  provider={p}
                  version={versionForProvider(p.name)}
                  loading={versionsLoading || updating === p.name}
                  updating={updating === p.name}
                  actionsDisabled={actionsDisabled}
                  enabledModels={getEnabledModels(settings, p.name)}
                  thinkingLevels={thinkingLevelsForProvider(settings.providerThinking ?? {}, p.name)}
                  swarmMode={settings.kimiSwarmEnabled ? "enabled" : ""}
                  ralphIterations={settings.kimiMaxRalphIterations === 1 ? "" : String(settings.kimiMaxRalphIterations)}
                  onToggleModel={(model) => handleToggleModel(p.name, model)}
                  onToggleAll={(selectAll) => handleToggleAll(p.name, selectAll)}
                  onUpdate={() => void handleUpdateCli(p.name)}
                  onThinkingChange={(model, value) => handleThinkingChange(p.name, model, value)}
                  onSwarmChange={handleSwarmChange}
                  onRalphIterationsChange={handleRalphIterationsChange}
                />
              ))}
            </div>
          )}
          {providers.length === 0 && !versionsLoading && (
            <p className="text-sm text-fg-faint">
              No providers detected. Ensure the Symphony sidecar is running.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
