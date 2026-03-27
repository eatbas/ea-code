import { useRef } from "react";
import { useUpdateInstall } from "./useUpdateInstall";
import { useUpdatePoller } from "./useUpdatePoller";

interface UpdateCheckState {
  status: "idle" | "queued" | "installing";
  updateVersion: string | null;
}

/**
 * Thin composition of update polling and installation.
 *
 * The poller checks for new versions on a 4-hour interval and on window focus.
 * The installer manages the download/queue/relaunch state machine.
 */
export function useUpdateCheck(updatesBlocked: boolean): UpdateCheckState {
  const install = useUpdateInstall(updatesBlocked);
  const updatesBlockedRef = useRef(updatesBlocked);
  updatesBlockedRef.current = updatesBlocked;

  useUpdatePoller({
    isInstalling: install.isInstalling,
    getAttemptedVersion: install.getAttemptedVersion,
    isBlocked: () => updatesBlockedRef.current,
    onInstall: install.installUpdate,
    onQueue: install.queueUpdate,
    onInstallPending: install.installPendingUpdate,
    onClose: install.closeUpdate,
    onDispose: install.dispose,
  });

  return { status: install.status, updateVersion: install.updateVersion };
}
