import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { SIDECAR_EVENTS } from "../constants/events";
import { checkSidecarReady } from "../lib/desktopApi";

interface SidecarReadyPayload {
  ready: boolean;
  error?: string;
}

interface UseSidecarReadyReturn {
  /** `null` while the sidecar is still starting up. */
  sidecarReady: boolean | null;
  sidecarError: string | null;
}

/**
 * Tracks whether the Symphony sidecar is healthy.
 *
 * Listens for the one-shot `sidecar_ready` event AND polls the backend on
 * mount so the UI recovers if the event fired before the listener registered.
 */
export function useSidecarReady(): UseSidecarReadyReturn {
  const [ready, setReady] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Listen for the Tauri event (handles the normal startup path).
    const unlistenPromise = listen<SidecarReadyPayload>(
      SIDECAR_EVENTS.READY,
      (event) => {
        setReady(event.payload.ready);
        setError(event.payload.error ?? null);
      },
    );

    // Also query the backend directly to cover the race where the event
    // already fired before this listener was registered.
    checkSidecarReady()
      .then((healthy) => {
        if (healthy) {
          setReady(true);
        }
      })
      .catch(() => {
        // Ignore — the event listener will catch it when the sidecar starts.
      });

    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  }, []);

  return {
    sidecarReady: ready,
    sidecarError: error,
  };
}
