import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { PipelineStatus } from "../types";

interface StatusBarProps {
  status: PipelineStatus;
  currentStage?: string;
  iteration?: number;
  maxIterations?: number;
  startedAt?: string;
}

/** Colour classes for each pipeline status. */
function statusBadgeClasses(status: PipelineStatus): string {
  switch (status) {
    case "running":
      return "bg-[#6366f1] text-white";
    case "completed":
      return "bg-[#22c55e] text-white";
    case "failed":
      return "bg-[#ef4444] text-white";
    case "cancelled":
      return "bg-[#f59e0b] text-white";
    default:
      return "bg-[#24243a] text-[#9898b0]";
  }
}

/** Format elapsed seconds into a human-readable string. */
function formatElapsed(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins > 0) {
    return `${mins}m ${secs}s`;
  }
  return `${secs}s`;
}

/** Bottom status bar showing pipeline state, stage, iteration, and elapsed time. */
export function StatusBar({
  status,
  currentStage,
  iteration,
  maxIterations,
  startedAt,
}: StatusBarProps): ReactNode {
  const [elapsed, setElapsed] = useState<number>(0);

  useEffect(() => {
    if (status !== "running" || !startedAt) {
      setElapsed(0);
      return;
    }

    const start = new Date(startedAt).getTime();

    function tick(): void {
      setElapsed(Math.floor((Date.now() - start) / 1000));
    }

    tick();
    const interval = setInterval(tick, 1000);

    return () => clearInterval(interval);
  }, [status, startedAt]);

  return (
    <footer className="bg-[#1a1a24] border-t border-[#2e2e48] px-4 py-2 flex items-center justify-between text-sm">
      <div className="flex items-center gap-3">
        <span className={`rounded px-2 py-0.5 text-xs font-medium ${statusBadgeClasses(status)}`}>
          {status.toUpperCase()}
        </span>

        {status === "running" && currentStage && (
          <span className="text-xs text-[#9898b0]">{currentStage}</span>
        )}
      </div>

      <div className="flex items-center gap-3">
        {status === "running" && iteration !== undefined && maxIterations !== undefined && (
          <span className="text-xs text-[#9898b0]">
            Iteration {iteration}/{maxIterations}
          </span>
        )}

        {status === "running" && (
          <span className="text-xs font-mono text-[#9898b0]">{formatElapsed(elapsed)}</span>
        )}
      </div>
    </footer>
  );
}
