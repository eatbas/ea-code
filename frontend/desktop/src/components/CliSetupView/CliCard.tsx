import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProviderInfo, ApiCliVersionInfo } from "../../types";
import { providerDisplayName, modelOptionsFromProvider, THINKING_OPTIONS } from "../shared/constants";
import { useToast } from "../shared/Toast";
import { ModelCheckboxList } from "./ModelCheckboxList";
import { ThinkingDropdown } from "./ThinkingDropdown";
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
  /** Current thinking / effort level for this provider (empty = default). */
  thinkingLevel: string;
  onToggleModel: (value: string) => void;
  onToggleAll: (selectAll: boolean) => void;
  onUpdate: () => void;
  /** Called when the user changes the thinking level. */
  onThinkingChange: (value: string) => void;
}

/** Card displaying a single CLI provider's version, models, and actions. */
export function CliCard({
  provider,
  version,
  loading,
  updating,
  actionsDisabled,
  enabledModels,
  thinkingLevel,
  onToggleModel,
  onToggleAll,
  onUpdate,
  onThinkingChange,
}: CliCardProps): ReactNode {
  const toast = useToast();
  const displayName = providerDisplayName(provider.name) + " CLI";
  const modelOptions = modelOptionsFromProvider(provider);
  const modelControlsDisabled = actionsDisabled || !provider.available;
  const thinkingOptions = THINKING_OPTIONS[provider.name];
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
          onToggleModel={onToggleModel}
          onToggleAll={onToggleAll}
        />
      )}
      {thinkingOptions && provider.available && (
        <ThinkingDropdown
          options={thinkingOptions}
          selected={thinkingLevel}
          disabled={modelControlsDisabled}
          onChange={onThinkingChange}
        />
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
