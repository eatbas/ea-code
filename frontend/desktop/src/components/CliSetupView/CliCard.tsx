import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProviderInfo, ApiCliVersionInfo } from "../../types";
import { providerDisplayName, modelOptionsFromProvider } from "../shared/constants";
import { useToast } from "../shared/Toast";
import { ModelCheckboxList } from "./ModelCheckboxList";
import { VersionGrid } from "./VersionGrid";

function buildGoogleInstallSearchUrl(name: string): string {
  const query = encodeURIComponent(`install ${name} CLI`);
  return `https://www.google.com/search?q=${query}`;
}

/** Version/install status badge. */
function StatusBadge({
  provider,
  version,
  loading,
}: {
  provider: ProviderInfo;
  version: ApiCliVersionInfo | undefined;
  loading: boolean;
}): ReactNode {
  if (loading) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#737373]/15 px-2.5 py-0.5 text-xs font-medium text-[#a3a3a3]">
        Checking...
      </span>
    );
  }
  if (!provider.available) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#ef4444]/15 px-2.5 py-0.5 text-xs font-medium text-[#ef4444]">
        Not Installed
      </span>
    );
  }
  if (!version?.latestVersion) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#737373]/15 px-2.5 py-0.5 text-xs font-medium text-[#a3a3a3]">
        Installed
      </span>
    );
  }
  if (version.upToDate) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#22c55e]/15 px-2.5 py-0.5 text-xs font-medium text-[#22c55e]">
        Up to Date
      </span>
    );
  }
  return (
    <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#f59e0b]/15 px-2.5 py-0.5 text-xs font-medium text-[#f59e0b]">
      Update Available
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
  onToggleModel: (value: string) => void;
  onToggleAll: (selectAll: boolean) => void;
  onUpdate: () => void;
}

/** Card displaying a single CLI provider's version, models, and actions. */
export function CliCard({
  provider,
  version,
  loading,
  updating,
  actionsDisabled,
  enabledModels,
  onToggleModel,
  onToggleAll,
  onUpdate,
}: CliCardProps): ReactNode {
  const toast = useToast();
  const displayName = providerDisplayName(provider.name) + " CLI";
  const modelOptions = modelOptionsFromProvider(provider);
  const modelControlsDisabled = actionsDisabled || !provider.available;
  const showUpdate =
    !loading && provider.available && version && !version.upToDate;
  const showInstall = !loading && !provider.available;
  const installSearchUrl = buildGoogleInstallSearchUrl(displayName);

  return (
    <div className="rounded-lg border border-[#313134] bg-[#151516] p-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#f5f5f5]">{displayName}</h3>
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
      {!loading && !provider.available && modelOptions.length > 0 && (
        <p className="mt-4 text-xs text-[#72727a]">
          Install this CLI to enable model selection.
        </p>
      )}
      {!loading && !provider.available && (
        <p className="mt-3 text-xs text-[#ef4444]">{provider.name} not found in PATH</p>
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
          className="mt-4 w-full rounded-md bg-[#f5f5f5] px-4 py-2 text-sm font-medium text-[#0b0b0c] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          Install
        </button>
      )}
      {showUpdate && (
        <button
          type="button"
          onClick={onUpdate}
          disabled={updating || actionsDisabled}
          className="mt-4 w-full rounded-md bg-[#f5f5f5] px-4 py-2 text-sm font-medium text-[#0b0b0c] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          {updating ? "Updating..." : "Update"}
        </button>
      )}
    </div>
  );
}
