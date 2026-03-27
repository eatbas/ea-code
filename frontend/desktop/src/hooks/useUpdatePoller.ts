import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { check, type Update } from "@tauri-apps/plugin-updater";

const FOUR_HOURS_MS = 4 * 60 * 60 * 1000;
const COOLDOWN_MS = 5 * 60 * 1000;

interface UpdatePollerCallbacks {
  /** Whether an install is currently in progress. */
  isInstalling: () => boolean;
  /** The version string of the last install attempt (avoids re-downloading). */
  getAttemptedVersion: () => string | null;
  /** Whether updates are currently blocked (e.g. pipeline running). */
  isBlocked: () => boolean;
  /** Called when a brand-new update should be installed immediately. */
  onInstall: (update: Update) => void;
  /** Called when an update should be queued for later. */
  onQueue: (update: Update) => void;
  /** Called to attempt installing a previously queued update. */
  onInstallPending: () => void;
  /** Release resources held by a downloaded update. */
  onClose: (update: Update | null) => void;
  /** Called on unmount to clean up. */
  onDispose: () => void;
}

/**
 * Periodically checks for app updates on a 4-hour interval and on window
 * focus, delegating install/queue decisions to the provided callbacks.
 */
export function useUpdatePoller(callbacks: UpdatePollerCallbacks): void {
  const lastCheckAtRef = useRef(0);
  const callbacksRef = useRef(callbacks);
  callbacksRef.current = callbacks;

  useEffect(() => {
    function runCheck(): void {
      const cbs = callbacksRef.current;
      if (cbs.isInstalling()) return;

      const now = Date.now();
      if (now - lastCheckAtRef.current < COOLDOWN_MS) return;
      lastCheckAtRef.current = now;

      void check()
        .then((update) => {
          if (!update?.available) return;

          const attempted = cbs.getAttemptedVersion();

          // Same version already queued — just try installing the pending one.
          if (attempted === update.version && cbs.isBlocked()) {
            cbs.onClose(update);
            cbs.onInstallPending();
            return;
          }

          // Already attempted this version and it failed — skip.
          if (attempted === update.version) {
            cbs.onClose(update);
            return;
          }

          if (cbs.isBlocked()) {
            cbs.onQueue(update);
            return;
          }

          cbs.onInstall(update);
        })
        .catch((err) => {
          console.warn("Update check failed:", err);
        });
    }

    runCheck();
    const interval = setInterval(runCheck, FOUR_HOURS_MS);
    const focusUnlisten = getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) {
        runCheck();
      }
    });

    return () => {
      callbacksRef.current.onDispose();
      clearInterval(interval);
      void focusUnlisten.then((unlisten) => unlisten());
    };
  }, []);
}
