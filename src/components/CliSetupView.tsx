import type { ReactNode } from "react";
import { useEffect } from "react";
import type { AppSettings, CliVersionInfo, AllCliVersions } from "../types";

interface CliSetupViewProps {
  settings: AppSettings;
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  onFetchVersions: (settings: AppSettings) => void;
  onUpdateCli: (cliName: string, settings: AppSettings) => void;
}

/** Status badge indicating whether a CLI is up-to-date, outdated, or missing. */
function StatusBadge({ info }: { info: CliVersionInfo }): ReactNode {
  if (!info.available) {
    return (
      <span className="inline-flex shrink-0 items-center whitespace-nowrap rounded-full bg-[#ef4444]/15 px-2.5 py-0.5 text-xs font-medium text-[#ef4444]">
        Not Installed
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

/** Single CLI tool card showing version information and update controls. */
function CliCard({
  info,
  updating,
  onUpdate,
}: {
  info: CliVersionInfo;
  updating: boolean;
  onUpdate: () => void;
}): ReactNode {
  const showUpdate = info.available && !info.upToDate;
  const showInstall = !info.available;

  return (
    <div className="rounded-lg border border-[#2e2e48] bg-[#1a1a2e] p-5">
      {/* Header row */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-[#e4e4ed]">{info.name}</h3>
        <StatusBadge info={info} />
      </div>

      {/* Version details */}
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

      {/* Error message */}
      {info.error && (
        <p className="mt-3 text-xs text-[#ef4444]">{info.error}</p>
      )}

      {/* Update / Install button */}
      {(showUpdate || showInstall) && (
        <button
          onClick={onUpdate}
          disabled={updating}
          className="mt-4 w-full rounded-md bg-[#6366f1] px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-[#5558e6] disabled:cursor-not-allowed disabled:opacity-50"
        >
          {updating
            ? "Updating…"
            : showInstall
              ? "Install"
              : "Update"}
        </button>
      )}
    </div>
  );
}

/** Card-based view for managing CLI tool versions. */
export function CliSetupView({
  settings,
  versions,
  loading,
  updating,
  error,
  onFetchVersions,
  onUpdateCli,
}: CliSetupViewProps): ReactNode {
  // Fetch version info on mount
  useEffect(() => {
    onFetchVersions(settings);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const cliEntries: CliVersionInfo[] = versions
    ? [versions.claude, versions.codex, versions.gemini]
    : [];

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {/* Header */}
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

          {/* Error banner */}
          {error && (
            <div className="rounded-md border border-[#ef4444]/30 bg-[#ef4444]/10 px-4 py-3 text-sm text-[#ef4444]">
              {error}
            </div>
          )}

          {/* Loading skeleton */}
          {loading && !versions && (
            <div className="grid gap-4 grid-cols-1 md:grid-cols-2">
              {[0, 1, 2].map((i) => (
                <div
                  key={i}
                  className="h-48 animate-pulse rounded-lg border border-[#2e2e48] bg-[#1a1a2e]"
                />
              ))}
            </div>
          )}

          {/* CLI cards */}
          {cliEntries.length > 0 && (
            <div className="grid gap-4 grid-cols-1 md:grid-cols-2">
              {cliEntries.map((info) => (
                <CliCard
                  key={info.cliName}
                  info={info}
                  updating={updating === info.cliName}
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
