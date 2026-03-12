import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { PipelineStage } from "../../types";
import { STAGE_LABELS } from "./constants";

interface ThinkingIndicatorProps {
  stage: PipelineStage;
  /** Absolute timestamp (Date.now()) when the stage started — survives remounts. */
  startedAt?: number;
}

/** Animated sweep indicator shown for the currently running pipeline stage. */
export function ThinkingIndicator({ stage, startedAt }: ThinkingIndicatorProps): ReactNode {
  const [fallbackElapsed, setFallbackElapsed] = useState(0);
  const [, tick] = useState(0);

  useEffect(() => {
    if (startedAt) {
      // Force re-render every second so the computed elapsed updates
      const interval = setInterval(() => tick((n) => n + 1), 1000);
      return () => clearInterval(interval);
    }
    // Fallback: local counter (used when startedAt is unavailable)
    setFallbackElapsed(0);
    const interval = setInterval(() => setFallbackElapsed((prev) => prev + 1), 1000);
    return () => clearInterval(interval);
  }, [stage, startedAt]);

  const elapsed = startedAt
    ? Math.max(0, Math.floor((Date.now() - startedAt) / 1000))
    : fallbackElapsed;

  const mins = Math.floor(elapsed / 60);
  const secs = elapsed % 60;
  const timer = mins > 0 ? `${mins}:${String(secs).padStart(2, "0")}` : `0:${String(secs).padStart(2, "0")}`;

  return (
    <div className="relative overflow-hidden rounded-lg border border-[#2e2e48] bg-[#161622] px-3 py-2">
      {/* Sweep overlay */}
      <div
        className="thinking-sweep-overlay pointer-events-none absolute inset-0"
      />
      <div className="relative z-10 flex items-center gap-2">
        <span className="text-xs font-semibold tracking-wide text-[#4ade80]">
          {STAGE_LABELS[stage] ?? stage}...
        </span>
        <span className="ml-auto text-[10px] tabular-nums text-[#9898b0] opacity-80">
          {timer}
        </span>
      </div>
    </div>
  );
}
