import { useState, useEffect } from "react";
import { formatDuration, parseUtcTimestamp } from "../utils/formatters";

/**
 * Ticks every second while the pipeline is running and
 * freezes elapsed time while paused.
 */
export function useElapsedTimer(
  status: string | undefined,
  startedAt: string | undefined,
  completedAt: string | undefined,
): string {
  const [, setTick] = useState(0);
  const [pausedElapsedMs, setPausedElapsedMs] = useState<number | null>(null);

  useEffect(() => {
    if (status !== "running") return;
    const interval = window.setInterval(() => setTick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [status]);

  useEffect(() => {
    if (!startedAt) {
      setPausedElapsedMs(null);
      return;
    }

    if (status === "paused" && !completedAt) {
      const startMs = parseUtcTimestamp(startedAt).getTime();
      if (Number.isFinite(startMs)) {
        setPausedElapsedMs(Math.max(0, Date.now() - startMs));
      }
      return;
    }

    if (status === "running") {
      setPausedElapsedMs(null);
    }
  }, [status, startedAt, completedAt]);

  if (!startedAt) return "0ms";
  const startMs = parseUtcTimestamp(startedAt).getTime();
  if (!Number.isFinite(startMs)) return "0ms";

  if (completedAt) {
    const endMs = parseUtcTimestamp(completedAt).getTime();
    return formatDuration(Math.max(0, endMs - startMs));
  }

  if (status === "paused" && pausedElapsedMs != null) {
    return formatDuration(pausedElapsedMs);
  }

  return formatDuration(Math.max(0, Date.now() - startMs));
}
