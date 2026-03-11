import { useState, useEffect } from "react";
import { formatDuration, parseUtcTimestamp } from "../utils/formatters";

/**
 * Ticks every second while the pipeline is running/paused and
 * returns a formatted elapsed-time string.
 */
export function useElapsedTimer(
  status: string | undefined,
  startedAt: string | undefined,
  completedAt: string | undefined,
): string {
  const [, setTick] = useState(0);

  useEffect(() => {
    if (status !== "running" && status !== "paused") return;
    const interval = window.setInterval(() => setTick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [status]);

  if (!startedAt) return "0ms";
  const startMs = parseUtcTimestamp(startedAt).getTime();
  if (!Number.isFinite(startMs)) return "0ms";
  const endMs = completedAt ? parseUtcTimestamp(completedAt).getTime() : Date.now();
  return formatDuration(Math.max(0, endMs - startMs));
}
