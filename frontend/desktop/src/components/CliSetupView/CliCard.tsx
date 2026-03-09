import type { ReactNode } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { CliVersionInfo } from "../../types";

function buildGoogleInstallSearchUrl(name: string): string {
  const query = encodeURIComponent(`install ${name}`);
  return `https://www.google.com/search?q=${query}`;
}

/** Version/install status badge for a CLI tool. */
function StatusBadge({ info }: { info: CliVersionInfo }): ReactNode {
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
  updating: boolean;
  enabledModels: Set<string>;
  modelOptions: { value: string; label: string }[];
  onToggleModel: (value: string) => void;
  onUpdate: () => void;
}

/** Card displaying a single CLI tool's version, models, and actions. */
export function CliCard({
  info,
  updating,
  enabledModels,
  modelOptions,
  onToggleModel,
  onUpdate,
}: CliCardProps): ReactNode {
  const showUpdate =
    info.available && !info.upToDate && info.updateCommand.trim().length > 0;
  const showInstall = !info.available;
  const installSearchUrl = buildGoogleInstallSearchUrl(info.name);

  return (
    <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-5">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#e4e4ed]">{info.name}</h3>
        <StatusBadge info={info} />
      </div>
      <div className="mt-4 grid grid-cols-2 gap-3">
        <div className="rounded-md bg-[#0f0f14] px-3 py-2">
          <p className="text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
            Installed
          </p>
          <p className="mt-0.5 text-sm font-mono text-[#e4e4ed]">
            {info.installedVersion ?? "—"}
          </p>
        </div>
        <div className="rounded-md bg-[#0f0f14] px-3 py-2">
          <p className="text-[10px] font-medium uppercase tracking-wider text-[#6b6b80]">
            Latest
          </p>
          <p className="mt-0.5 text-sm font-mono text-[#e4e4ed]">
            {info.latestVersion ?? "—"}
          </p>
        </div>
      </div>
      {info.available && modelOptions.length > 0 && (
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
                  className={`flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
                    isChecked
                      ? "bg-[#24243a] text-[#e4e4ed]"
                      : "bg-[#0f0f14] text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                  }`}
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
      {!info.available && modelOptions.length > 0 && (
        <p className="mt-4 text-xs text-[#6b6b80]">
          Install this CLI to enable model selection.
        </p>
      )}
      {info.error && <p className="mt-3 text-xs text-[#ef4444]">{info.error}</p>}
      {showInstall && (
        <button
          onClick={() => void openUrl(installSearchUrl)}
          className="mt-4 w-full rounded-md bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] transition-colors hover:bg-white"
        >
          Install
        </button>
      )}
      {showUpdate && (
        <button
          onClick={onUpdate}
          disabled={updating}
          className="mt-4 w-full rounded-md bg-[#e4e4ed] px-4 py-2 text-sm font-medium text-[#0f0f14] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
        >
          {updating ? "Updating…" : "Update"}
        </button>
      )}
    </div>
  );
}
