import type { ReactNode } from "react";
import { useEffect } from "react";
import type { AppSettings, CliVersionInfo, AllCliVersions } from "../types";
import { CLI_MODEL_OPTIONS } from "../types";

/** Settings key for each CLI's enabled-models field (comma-separated). */
type ModelSettingsKey =
  | "claudeModel"
  | "codexModel"
  | "geminiModel"
  | "kimiModel"
  | "opencodeModel";

/** Map from CLI name to its settings key. */
const MODEL_KEY_MAP: Record<string, ModelSettingsKey> = {
  claude: "claudeModel",
  codex: "codexModel",
  gemini: "geminiModel",
  kimi: "kimiModel",
  opencode: "opencodeModel",
};

interface CliSetupViewProps {
  settings: AppSettings;
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  onFetchVersions: (settings: AppSettings) => void;
  onUpdateCli: (cliName: string, settings: AppSettings) => void;
  onSave: (settings: AppSettings) => void;
}

/** Parses a comma-separated model string into a Set. */
function parseEnabledModels(csv: string): Set<string> {
  return new Set(csv.split(",").map((s) => s.trim()).filter(Boolean));
}

/** Serialises a Set of model values back to a comma-separated string. */
function serialiseEnabledModels(models: Set<string>): string {
  return Array.from(models).join(",");
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

/** Single CLI tool card showing version info, multi-model checklist, and update controls. */
function CliCard({
  info,
  updating,
  enabledModels,
  modelOptions,
  onToggleModel,
  onUpdate,
}: {
  info: CliVersionInfo;
  updating: boolean;
  enabledModels: Set<string>;
  modelOptions: { value: string; label: string }[];
  onToggleModel: (value: string) => void;
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

      {/* Model checklist (multi-select) */}
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
                  className={`flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
                    isChecked
                      ? "bg-[#6366f1]/15 text-[#e4e4ed]"
                      : "bg-[#0f0f14] text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                  }`}
                >
                  <span
                    className={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                      isChecked
                        ? "border-[#6366f1] bg-[#6366f1]"
                        : "border-[#3e3e58] bg-transparent"
                    }`}
                  >
                    {isChecked && (
                      <svg
                        className="h-3 w-3 text-white"
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

/** Card-based view for managing CLI tool versions and model selection. */
export function CliSetupView({
  settings,
  versions,
  loading,
  updating,
  error,
  onFetchVersions,
  onUpdateCli,
  onSave,
}: CliSetupViewProps): ReactNode {
  // Fetch version info on mount
  useEffect(() => {
    onFetchVersions(settings);
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const cliEntries: CliVersionInfo[] = versions
    ? [versions.claude, versions.codex, versions.gemini, versions.kimi, versions.opencode]
    : [];

  /** Returns the set of currently enabled models for a CLI. */
  function getEnabledModels(cliName: string): Set<string> {
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return new Set();
    return parseEnabledModels(settings[key]);
  }

  /** Toggles a model on/off for a given CLI. At least one model must remain enabled. */
  function handleToggleModel(cliName: string, model: string): void {
    const key = MODEL_KEY_MAP[cliName];
    if (!key) return;
    const current = parseEnabledModels(settings[key]);

    if (current.has(model)) {
      // Prevent disabling the last enabled model
      if (current.size <= 1) return;
      current.delete(model);
    } else {
      current.add(model);
    }

    const updated = { ...settings, [key]: serialiseEnabledModels(current) };
    onSave(updated);
  }

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
              {[0, 1, 2, 3, 4].map((i) => (
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
                  enabledModels={getEnabledModels(info.cliName)}
                  modelOptions={CLI_MODEL_OPTIONS[info.cliName] ?? []}
                  onToggleModel={(model) => handleToggleModel(info.cliName, model)}
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
