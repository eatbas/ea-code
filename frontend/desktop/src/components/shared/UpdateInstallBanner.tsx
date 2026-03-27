import type { ReactNode } from "react";

interface UpdateInstallBannerProps {
  mode: "queued" | "installing";
  version: string | null;
}

/** Minimal status indicator displayed while an app update is being installed. */
export function UpdateInstallBanner({ mode, version }: UpdateInstallBannerProps): ReactNode {
  const text = mode === "queued"
    ? `Update ready: v${version ?? "latest"}. Waiting for the current session to finish.`
    : `Updating to v${version ?? "latest"}...`;

  return (
    <div className="pointer-events-none fixed bottom-3 right-3 z-50 rounded border border-[#313134] bg-[#151516]/95 px-3 py-1 text-xs text-[#f5f5f5] shadow-lg">
      {text}
    </div>
  );
}
