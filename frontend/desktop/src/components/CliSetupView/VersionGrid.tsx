import type { ReactNode } from "react";
import type { ApiCliVersionInfo } from "../../types";

interface VersionGridProps {
  /** Version metadata for this provider (may be undefined while loading). */
  version: ApiCliVersionInfo | undefined;
  /** Whether version data is currently being fetched. */
  loading: boolean;
}

/** Two-column grid showing installed and latest CLI version. */
export function VersionGrid({ version, loading }: VersionGridProps): ReactNode {
  return (
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
  );
}
