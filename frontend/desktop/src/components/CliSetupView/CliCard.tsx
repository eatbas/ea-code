import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProviderInfo, ApiCliVersionInfo } from "../../types";
import {
  providerDisplayName,
  modelOptionsFromProvider,
  getThinkingOptions,
  THINKING_TRIGGER_LABELS,
  SWARM_OPTIONS,
  RALPH_ITERATIONS_OPTIONS,
  RALPH_TRIGGER_LABELS,
} from "../shared/constants";
import { PopoverSelect } from "../shared/PopoverSelect";
import { useToast } from "../shared/Toast";
import { ModelCheckboxList } from "./ModelCheckboxList";
import { VersionGrid } from "./VersionGrid";

function buildGoogleInstallSearchUrl(name: string): string {
  const query = encodeURIComponent(`install ${name} CLI`);
  return `https://www.google.com/search?q=${query}`;
}

const BADGE_BASE = "inline-flex shrink-0 items-center whitespace-nowrap rounded-full px-2.5 py-0.5 text-xs font-medium";

const BADGE_VARIANTS = {
  neutral: "bg-neutral-badge-bg text-neutral-badge-text",
  danger: "bg-danger/15 text-danger",
  success: "bg-success/15 text-success",
  warning: "bg-warning/15 text-warning",
} as const;

function StatusBadge({
  provider,
  version,
  loading,
}: {
  provider: ProviderInfo;
  version: ApiCliVersionInfo | undefined;
  loading: boolean;
}): ReactNode {
  let label: string;
  let variant: keyof typeof BADGE_VARIANTS;

  if (loading) {
    label = "Checking...";
    variant = "neutral";
  } else if (!provider.available) {
    label = "Not Installed";
    variant = "danger";
  } else if (!version?.latestVersion) {
    label = "Installed";
    variant = "neutral";
  } else if (version.upToDate) {
    label = "Up to Date";
    variant = "success";
  } else {
    label = "Update Available";
    variant = "warning";
  }

  return (
    <span className={`${BADGE_BASE} ${BADGE_VARIANTS[variant]}`}>
      {label}
    </span>
  );
}

interface CliCardProps {
  provider: ProviderInfo;
  version: ApiCliVersionInfo | undefined;
  loading: boolean;
  updating: boolean;
  actionsDisabled: boolean;
  enabledModels: Set<string>;
  /** Per-model thinking levels keyed by "provider:model". */
  thinkingLevels: Record<string, string>;
  /** Kimi swarm mode value ("enabled" or ""). */
  swarmMode: string;
  /** Kimi ralph iterations value (string or ""). */
  ralphIterations: string;
  onToggleModel: (value: string) => void;
  onToggleAll: (selectAll: boolean) => void;
  onUpdate: () => void;
  /** Called when the thinking level changes for a specific model. */
  onThinkingChange: (model: string, value: string) => void;
  /** Called when swarm mode changes. */
  onSwarmChange: (value: string) => void;
  /** Called when ralph iterations changes. */
  onRalphIterationsChange: (value: string) => void;
}

/** Card displaying a single CLI provider's version, models, and actions. */
export function CliCard({
  provider,
  version,
  loading,
  updating,
  actionsDisabled,
  enabledModels,
  thinkingLevels,
  swarmMode,
  ralphIterations,
  onToggleModel,
  onToggleAll,
  onUpdate,
  onThinkingChange,
  onSwarmChange,
  onRalphIterationsChange,
}: CliCardProps): ReactNode {
  const toast = useToast();
  const displayName = providerDisplayName(provider.name) + " CLI";
  const modelOptions = modelOptionsFromProvider(provider);
  const modelControlsDisabled = actionsDisabled || !provider.available;
  const thinkingOptions: Record<string, { value: string; label: string }[]> | undefined =
    (() => {
      const map: Record<string, { value: string; label: string }[]> = {};
      let hasAny = false;
      for (const m of provider.models) {
        const opts = getThinkingOptions(provider.name, m);
        if (opts) {
          map[m] = opts;
          hasAny = true;
        }
      }
      return hasAny ? map : undefined;
    })();
  const thinkingTriggerLabels = THINKING_TRIGGER_LABELS[provider.name];
  const showUpdate =
    !loading && provider.available && version && !version.upToDate;
  const showInstall = !loading && !provider.available;
  const installSearchUrl = buildGoogleInstallSearchUrl(displayName);

  return (
    <div className="rounded-lg border border-edge bg-panel p-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-fg">{displayName}</h3>
        <StatusBadge provider={provider} version={version} loading={loading} />
      </div>
      <VersionGrid version={version} loading={loading} />
      {modelOptions.length > 0 && (
        <ModelCheckboxList
          modelOptions={modelOptions}
          enabledModels={enabledModels}
          disabled={modelControlsDisabled}
          thinkingOptions={provider.available ? thinkingOptions : undefined}
          thinkingLevels={thinkingLevels}
          thinkingTriggerLabels={provider.available ? thinkingTriggerLabels : undefined}
          onToggleModel={onToggleModel}
          onToggleAll={onToggleAll}
          onThinkingChange={onThinkingChange}
        />
      )}
      {provider.name === "kimi" && provider.available && (
        <div className="mt-4 flex flex-col gap-3">
          <div>
            <p className="mb-2 text-[10px] font-medium uppercase tracking-wider text-fg-faint">
              Swarm Mode
            </p>
            <PopoverSelect
              value={swarmMode}
              options={SWARM_OPTIONS}
              onChange={onSwarmChange}
              disabled={modelControlsDisabled}
              direction="down"
              placeholder="Disabled"
              triggerClassName="flex w-full h-10 items-center gap-2 rounded-md border border-edge-strong bg-input-bg px-3 text-sm font-medium text-fg shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55"
              menuClassName="w-full min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur"
            />
          </div>
          {swarmMode === "enabled" && (
            <div>
              <p className="mb-2 text-[10px] font-medium uppercase tracking-wider text-fg-faint">
                Ralph Iterations
              </p>
              <PopoverSelect
                value={ralphIterations}
                options={RALPH_ITERATIONS_OPTIONS}
                onChange={onRalphIterationsChange}
                disabled={modelControlsDisabled}
                direction="down"
                placeholder="Default"
                triggerLabels={RALPH_TRIGGER_LABELS}
                triggerClassName="flex w-full h-10 items-center gap-2 rounded-md border border-edge-strong bg-input-bg px-3 text-sm font-medium text-fg shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55"
                menuClassName="w-full min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur"
              />
            </div>
          )}
        </div>
      )}
      {!loading && !provider.available && modelOptions.length > 0 && (
        <p className="mt-4 text-xs text-fg-faint">
          Install this CLI to enable model selection.
        </p>
      )}
      {!loading && !provider.available && (
        <p className="mt-3 text-xs text-danger">{provider.name} not found in PATH</p>
      )}
      {showInstall && (
        <button
          type="button"
          onClick={() => {
            void openUrl(installSearchUrl).catch(() => {
              toast.error(`Could not open install page for ${displayName}.`);
            });
          }}
          disabled={actionsDisabled}
          className="mt-4 w-full rounded-md bg-fg px-4 py-2 text-sm font-medium text-surface transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          Install
        </button>
      )}
      {showUpdate && (
        <button
          type="button"
          onClick={onUpdate}
          disabled={updating || actionsDisabled}
          className="mt-4 w-full rounded-md bg-fg px-4 py-2 text-sm font-medium text-surface transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          {updating ? "Updating..." : "Update"}
        </button>
      )}
    </div>
  );
}
