import type { ReactNode } from "react";

interface UpdateInstallBannerProps {
  version: string | null;
}

/** Minimal status indicator displayed while an app update is being installed. */
export function UpdateInstallBanner({ version }: UpdateInstallBannerProps): ReactNode {
  return (
    <div className="pointer-events-none fixed bottom-3 right-3 z-50 rounded border border-[#2e2e48] bg-[#1a1a24]/95 px-3 py-1 text-xs text-[#e4e4ed] shadow-lg">
      {`Updating to v${version ?? "latest"}...`}
    </div>
  );
}
