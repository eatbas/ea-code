import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useToast } from "../components/shared/Toast";

const FOUR_HOURS_MS = 4 * 60 * 60 * 1000;
const COOLDOWN_MS = 5 * 60 * 1000;

interface UpdateCheckState {
  installing: boolean;
  updateVersion: string | null;
}

export function useUpdateCheck(): UpdateCheckState {
  const toast = useToast();
  const [installing, setInstalling] = useState(false);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);

  const lastCheckAtRef = useRef(0);
  const installingRef = useRef(false);
  const attemptedVersionRef = useRef<string | null>(null);
  const checkFailureNotifiedRef = useRef(false);

  useEffect(() => {
    function installUpdate(update: Update): void {
      if (installingRef.current) return;
      if (attemptedVersionRef.current === update.version) return;

      installingRef.current = true;
      attemptedVersionRef.current = update.version;
      setUpdateVersion(update.version);
      setInstalling(true);

      void update
        .downloadAndInstall()
        .then(() => relaunch())
        .catch(() => {
          installingRef.current = false;
          attemptedVersionRef.current = null;
          setInstalling(false);
          toast.error("Failed to install the update.");
        });
    }

    function runCheck(): void {
      if (installingRef.current) return;

      const now = Date.now();
      if (now - lastCheckAtRef.current < COOLDOWN_MS) return;
      lastCheckAtRef.current = now;

      void check()
        .then((update) => {
          checkFailureNotifiedRef.current = false;
          if (update?.available) installUpdate(update);
        })
        .catch(() => {
          // Keep this low-noise: notify once until a successful check occurs.
          if (!checkFailureNotifiedRef.current) {
            checkFailureNotifiedRef.current = true;
            toast.error("Failed to check for updates.");
          }
        });
    }

    runCheck();
    const interval = setInterval(runCheck, FOUR_HOURS_MS);
    const focusUnlisten = getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) runCheck();
    });

    return () => {
      clearInterval(interval);
      void focusUnlisten.then((unlisten) => unlisten());
    };
  }, [toast]);

  return { installing, updateVersion };
}
