import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const POLL_INTERVAL_MS = 3000;

/** Polls the backend for any persisted live sessions across all projects. */
export function useLiveSessionStatus(): boolean {
  const [hasLiveSessions, setHasLiveSessions] = useState(false);

  useEffect(() => {
    let disposed = false;

    async function refresh(): Promise<void> {
      try {
        const liveSessions = await invoke<boolean>("has_live_sessions");
        if (!disposed) {
          setHasLiveSessions(liveSessions);
        }
      } catch {
        if (!disposed) {
          setHasLiveSessions(false);
        }
      }
    }

    void refresh();
    const interval = setInterval(() => {
      void refresh();
    }, POLL_INTERVAL_MS);
    const focusUnlisten = getCurrentWindow().onFocusChanged(({ payload }) => {
      if (payload) {
        void refresh();
      }
    });

    return () => {
      disposed = true;
      clearInterval(interval);
      void focusUnlisten.then((unlisten) => unlisten());
    };
  }, []);

  return hasLiveSessions;
}
