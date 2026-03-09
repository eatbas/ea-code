import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

const FOUR_HOURS_MS = 4 * 60 * 60 * 1000;
const COOLDOWN_MS = 5 * 60 * 1000;

interface UpdateCheckState {
  installing: boolean;
  updateVersion: string | null;
}

export function useUpdateCheck(): UpdateCheckState {
  const [installing, setInstalling] = useState(false);
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);

  const lastCheckAtRef = useRef(0);
  const installingRef = useRef(false);
  const attemptedVersionRef = useRef<string | null>(null);

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
        });
    }

    function runCheck(): void {
      if (installingRef.current) return;

      const now = Date.now();
      if (now - lastCheckAtRef.current < COOLDOWN_MS) return;
      lastCheckAtRef.current = now;

      void check()
        .then((update) => {
          if (update?.available) installUpdate(update);
        })
        .catch(() => {
          // Ignore transient updater errors and retry on next scheduled check.
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
  }, []);

  return { installing, updateVersion };
}
