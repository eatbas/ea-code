import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useToast } from "../components/shared/Toast";

const FOUR_HOURS_MS = 4 * 60 * 60 * 1000;
const COOLDOWN_MS = 5 * 60 * 1000;

interface UpdateCheckState {
  status: "idle" | "queued" | "installing";
  updateVersion: string | null;
}

export function useUpdateCheck(updatesBlocked: boolean): UpdateCheckState {
  const toast = useToast();
  const [status, setStatus] = useState<"idle" | "queued" | "installing">("idle");
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);

  const lastCheckAtRef = useRef(0);
  const installingRef = useRef(false);
  const attemptedVersionRef = useRef<string | null>(null);
  const checkFailureNotifiedRef = useRef(false);
  const pendingUpdateRef = useRef<Update | null>(null);
  const updatesBlockedRef = useRef(updatesBlocked);

  updatesBlockedRef.current = updatesBlocked;

  function setIdleState(): void {
    setStatus("idle");
    setUpdateVersion(null);
  }

  function closeUpdate(update: Update | null): void {
    if (!update) return;
    void update.close().catch(() => {
      // Ignore close failures; they do not affect update flow.
    });
  }

  function installUpdate(update: Update): void {
    if (installingRef.current) return;

    installingRef.current = true;
    attemptedVersionRef.current = update.version;
    pendingUpdateRef.current = null;
    setUpdateVersion(update.version);
    setStatus("installing");

    void update
      .downloadAndInstall()
      .then(() => relaunch())
      .catch(() => {
        installingRef.current = false;
        attemptedVersionRef.current = null;
        setIdleState();
        closeUpdate(update);
        toast.error("Failed to install the update.");
      });
  }

  function queueUpdate(update: Update): void {
    const currentPending = pendingUpdateRef.current;
    if (currentPending?.version === update.version) {
      closeUpdate(update);
      return;
    }

    closeUpdate(currentPending);
    pendingUpdateRef.current = update;
    attemptedVersionRef.current = update.version;
    setUpdateVersion(update.version);
    setStatus("queued");
  }

  function installPendingUpdate(): void {
    const pendingUpdate = pendingUpdateRef.current;
    if (!pendingUpdate || updatesBlockedRef.current) return;
    installUpdate(pendingUpdate);
  }

  useEffect(() => {
    function runCheck(): void {
      if (installingRef.current) return;

      const now = Date.now();
      if (now - lastCheckAtRef.current < COOLDOWN_MS) return;
      lastCheckAtRef.current = now;

      void check()
        .then((update) => {
          checkFailureNotifiedRef.current = false;
          if (!update?.available) return;

          const pendingVersion = pendingUpdateRef.current?.version;
          if (pendingVersion === update.version) {
            closeUpdate(update);
            installPendingUpdate();
            return;
          }

          if (attemptedVersionRef.current === update.version) {
            closeUpdate(update);
            return;
          }

          if (updatesBlockedRef.current) {
            queueUpdate(update);
            return;
          }

          installUpdate(update);
        })
        .catch(() => {
          if (!checkFailureNotifiedRef.current) {
            checkFailureNotifiedRef.current = true;
            toast.error("Failed to check for updates.");
          }
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
      closeUpdate(pendingUpdateRef.current);
      pendingUpdateRef.current = null;
      clearInterval(interval);
      void focusUnlisten.then((unlisten) => unlisten());
    };
  }, [toast]);

  useEffect(() => {
    if (!updatesBlocked) {
      installPendingUpdate();
    }
  }, [updatesBlocked]);

  return { status, updateVersion };
}
