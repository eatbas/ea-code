import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { SIDECAR_EVENTS } from "../constants/events";
import { getSidecarLogs } from "../lib/desktopApi";
import { disposeTauriListener } from "../utils/tauriListeners";
import type { SidecarLogEntry } from "../types";

export interface UseSidecarLogsReturn {
  /** Formatted log text for display. */
  logs: string;
  /** Clear accumulated logs. */
  clearLogs: () => void;
}

function formatEntry(entry: SidecarLogEntry): string {
  const ts = entry.timestamp.replace("T", " ").replace(/\.\d+.*$/, "");
  return `[${ts}] [${entry.stream}] ${entry.line}`;
}

/**
 * Streams sidecar stdout/stderr logs via Tauri events and back-fills
 * buffered entries on mount (same race-recovery pattern as useSidecarReady).
 */
export function useSidecarLogs(): UseSidecarLogsReturn {
  const [logs, setLogs] = useState("");
  const backfilled = useRef(false);

  useEffect(() => {
    const unlistenPromise = listen<SidecarLogEntry>(
      SIDECAR_EVENTS.LOG,
      (event) => {
        const line = formatEntry(event.payload);
        setLogs((prev) => (prev ? `${prev}\n${line}` : line));
      },
    );

    if (!backfilled.current) {
      backfilled.current = true;
      getSidecarLogs()
        .then((entries) => {
          if (entries.length > 0) {
            const text = entries.map(formatEntry).join("\n");
            setLogs((prev) => (prev ? `${text}\n${prev}` : text));
          }
        })
        .catch(() => {
          // The event listener will catch future logs.
        });
    }

    return () => {
      disposeTauriListener(unlistenPromise, SIDECAR_EVENTS.LOG);
    };
  }, []);

  const clearLogs = useCallback(() => {
    setLogs("");
  }, []);

  return { logs, clearLogs };
}
