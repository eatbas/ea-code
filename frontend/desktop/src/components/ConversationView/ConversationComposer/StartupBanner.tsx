import type { ReactNode } from "react";
import { AlertCircle, RefreshCw, TerminalSquare } from "lucide-react";
import type { SymphonyStartupStatus } from "../../../utils/symphonyStartup";

interface StartupBannerProps {
  status: SymphonyStartupStatus;
  onOpenCliSetup: () => void;
}

export function StartupBanner({
  status,
  onOpenCliSetup,
}: StartupBannerProps): ReactNode {
  if (status.phase === "connected") {
    return null;
  }

  if (status.phase === "failed") {
    return (
      <div className="mx-3 mt-3 flex flex-col gap-3 rounded-2xl border border-danger/30 bg-danger/10 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-start gap-3">
          <AlertCircle size={16} strokeWidth={2} className="mt-0.5 shrink-0 text-danger" />
          <div>
            <p className="text-sm font-semibold text-fg">Symphony is unavailable</p>
            <p className="mt-1 text-xs text-fg-muted">
              {status.errorMessage ?? "Maestro could not finish the startup checks."}
            </p>
          </div>
        </div>
        <button
          type="button"
          onClick={onOpenCliSetup}
          className="inline-flex items-center gap-2 self-start rounded-lg border border-edge bg-elevated px-3 py-2 text-xs font-semibold text-fg transition-colors hover:bg-active"
        >
          <TerminalSquare size={13} strokeWidth={2} />
          Open CLI Setup
        </button>
      </div>
    );
  }

  const body = status.phase === "initialising"
    ? "Starting Symphony. You can type now, and send will unlock automatically when startup completes."
    : "Checking available agents and CLI version data. Send will unlock automatically when the checks finish.";

  return (
    <div className="mx-3 mt-3 flex items-start gap-3 rounded-2xl border border-edge bg-elevated px-4 py-3">
      <RefreshCw size={16} strokeWidth={2} className="mt-0.5 shrink-0 animate-spin text-fg-muted" />
      <div>
        <p className="text-sm font-semibold text-fg">
          {status.phase === "initialising" ? "Initialising Symphony" : "Checking Symphony"}
        </p>
        <p className="mt-1 text-xs text-fg-muted">{body}</p>
      </div>
    </div>
  );
}
