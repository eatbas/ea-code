import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { CliVersionInfo } from "../../types";
import { useToast } from "../shared/Toast";

function buildGoogleInstallSearchUrl(name: string): string {
  const query = encodeURIComponent(`install ${name}`);
  return `https://www.google.com/search?q=${query}`;
}

/** Version/install status badge for a CLI tool. */
function StatusBadge({ info, loading }: { info: CliVersionInfo; loading: boolean }): ReactNode {
  if (loading) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#64748b]/15 px-2.5 py-0.5 text-xs font-medium text-[#94a3b8]">
        Checking...
      </span>
    );
  }
  if (!info.available) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#ef4444]/15 px-2.5 py-0.5 text-xs font-medium text-[#ef4444]">
        Not Installed
      </span>
    );
  }
  if (!info.latestVersion) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#64748b]/15 px-2.5 py-0.5 text-xs font-medium text-[#94a3b8]">
        Installed
      </span>
    );
  }
  if (info.upToDate) {
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
  info: CliVersionInfo;
  loading: boolean;
  updating: boolean;
  actionsDisabled: boolean;
  enabledModels: Set<string>;
  modelOptions: { value: string; label: string }[];
  onToggleModel: (value: string) => void;
  onUpdate: () => void;
}

/** Card displaying a single CLI tool's version, models, and actions. */
export function CliCard({
  info,
  loading,
  updating,
  actionsDisabled,
  enabledModels,
  modelOptions,
  onToggleModel,
  onUpdate,
}: CliCardProps): ReactNode {
  const toast = useToast();
  const modelControlsDisabled = actionsDisabled || !info.available;
  const showUpdate =
    !loading && info.available && !info.upToDate && info.updateCommand.trim().length > 0;
  const showInstall = !loading && !info.available;
  const installSearchUrl = buildGoogleInstallSearchUrl(info.name);

  return (
    <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#e4e4ed]">{info.name}</h3>
        <StatusBadge info={info} loading={loading} />
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
              {info.installedVersion ?? "N/A"}
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
              {info.latestVersion ?? "N/A"}
            </p>
          )}
        </div>
      </div>
      {modelOptions.length > 0 && (
        <div className="mt-4">
          <p className="mb-2 text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
            Models
          </p>
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
      )}
      {!loading && !info.available && modelOptions.length > 0 && (
        <p className="mt-4 text-xs text-[#6b6b80]">
          Install this CLI to enable model selection.
        </p>
      )}
      {!loading && info.error && <p className="mt-3 text-xs text-[#ef4444]">{info.error}</p>}
      {showInstall && (
        <button
          type="button"
          onClick={() => {
            void openUrl(installSearchUrl).catch(() => {
              toast.error(`Could not open install page for ${info.name}.`);
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
          {updating
            ? "Updating..."
            : info.cliName === "gitBash"
              ? "Download Update"
              : "Update"}
        </button>
      )}
    </div>
  );
}
