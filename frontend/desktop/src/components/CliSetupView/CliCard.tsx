import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { ProviderInfo, ApiCliVersionInfo } from "../../types";
import { providerDisplayName, modelOptionsFromProvider } from "../shared/constants";
import { useToast } from "../shared/Toast";

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
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#64748b]/15 px-2.5 py-0.5 text-xs font-medium text-[#94a3b8]">
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
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#64748b]/15 px-2.5 py-0.5 text-xs font-medium text-[#94a3b8]">
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
    <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#e4e4ed]">{displayName}</h3>
        <StatusBadge provider={provider} version={version} loading={loading} />
      </div>
      <div className="mt-4 grid grid-cols-2 gap-3">
        <div className="rounded-md bg-[#0f0f14] px-3 py-2">
          <p className="text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
            Installed
          </p>
          {loading ? (
            <div className="mt-1 h-4 w-20 animate-pulse rounded bg-[#24243a]" />
          ) : (
            <p className="mt-0.5 text-sm font-mono text-[#e4e4ed]">
              {version?.installedVersion ?? "N/A"}
            </p>
          )}
        </div>
        <div className="rounded-md bg-[#0f0f14] px-3 py-2">
          <p className="text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
            Latest
          </p>
          {loading ? (
            <div className="mt-1 h-4 w-20 animate-pulse rounded bg-[#24243a]" />
          ) : (
            <p className="mt-0.5 text-sm font-mono text-[#e4e4ed]">
              {version?.latestVersion ?? "N/A"}
            </p>
          )}
        </div>
      </div>
      {modelOptions.length > 0 && (() => {
        const allSelected = modelOptions.every((opt) => enabledModels.has(opt.value));
        return (
        <div className="mt-4">
          <div className="mb-2 flex items-center justify-between">
            <p className="text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
              Models
            </p>
            <button
              type="button"
              onClick={() => onToggleAll(!allSelected)}
              disabled={modelControlsDisabled}
              className="flex items-center gap-1.5 text-[10px] font-medium text-[#6b6b80] transition-colors hover:text-[#e4e4ed] disabled:cursor-not-allowed disabled:opacity-50"
            >
              <span
                className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border ${
                  allSelected
                    ? "border-[#e4e4ed] bg-[#e4e4ed]"
                    : "border-[#3e3e58] bg-transparent"
                }`}
              >
                {allSelected && (
                  <svg
                    className="h-2.5 w-2.5 text-[#0f0f14]"
                    viewBox="0 0 12 12"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M2.5 6L5 8.5L9.5 3.5" />
                  </svg>
                )}
              </span>
              Select all
            </button>
          </div>
          <div className="flex flex-col gap-1.5">
            {modelOptions.map((opt) => {
              const isChecked = enabledModels.has(opt.value);
              return (
                <button
                  key={opt.value}
                  type="button"
                  onClick={() => onToggleModel(opt.value)}
                  disabled={modelControlsDisabled}
                  className={`flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
                    isChecked
                      ? "bg-[#24243a] text-[#e4e4ed]"
                      : modelControlsDisabled
                        ? "bg-[#0f0f14] text-[#6b6b80]"
                        : "bg-[#0f0f14] text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                  } disabled:cursor-not-allowed disabled:opacity-50`}
                >
                  <span
                    className={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                      isChecked
                        ? "border-[#e4e4ed] bg-[#e4e4ed]"
                        : "border-[#3e3e58] bg-transparent"
                    }`}
                  >
                    {isChecked && (
                      <svg
                        className="h-3 w-3 text-[#0f0f14]"
                        viewBox="0 0 12 12"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      >
                        <path d="M2.5 6L5 8.5L9.5 3.5" />
                      </svg>
                    )}
                  </span>
                  {opt.label}
                </button>
              );
            })}
          </div>
        </div>
        );
      })()}
      {!loading && !provider.available && modelOptions.length > 0 && (
        <p className="mt-4 text-xs text-[#6b6b80]">
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
          className="mt-4 w-full rounded-md bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          Install
        </button>
      )}
      {showUpdate && (
        <button
          type="button"
          onClick={onUpdate}
          disabled={updating || actionsDisabled}
          className="mt-4 w-full rounded-md bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          {updating ? "Updating..." : "Update"}
        </button>
      )}
    </div>
  );
}
