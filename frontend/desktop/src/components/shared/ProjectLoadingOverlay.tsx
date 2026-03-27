import type { ReactNode } from "react";

/** Full-screen project loading state shown while switching/opening workspaces. */
export function ProjectLoadingOverlay(): ReactNode {
  return (
    <div className="absolute inset-0 z-50 flex items-center justify-center bg-black/35">
      <div className="flex min-w-[230px] flex-col items-center gap-3 rounded-2xl border border-edge bg-[#111112]/95 px-8 py-7 shadow-2xl">
        <img
          src="/logo.png"
          alt="EA Code logo"
          className="h-12 w-12 object-contain"
        />
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-edge border-t-fg" />
        <p className="text-sm text-[#d0d0d4]">Opening project...</p>
      </div>
    </div>
  );
}
