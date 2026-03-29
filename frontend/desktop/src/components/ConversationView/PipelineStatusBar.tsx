import type { ReactNode } from "react";
import { useEffect, useState } from "react";

interface PipelineStatusBarProps {
  /** Name of the currently active stage (e.g. "Planner 1", "Code Fix"). */
  stageName: string;
  /** Whether the pipeline is actively running. */
  running: boolean;
  /** Epoch ms when the pipeline started. */
  startedAt: number;
  /** Epoch ms when the pipeline finished (stops the total timer). */
  finishedAt?: number;
}

function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${String(minutes)}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${String(seconds)}s`;
}

export function PipelineStatusBar({
  stageName,
  running,
  startedAt,
  finishedAt,
}: PipelineStatusBarProps): ReactNode {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [running]);

  const elapsed = formatElapsed((finishedAt ?? now) - startedAt);

  return (
    <div className="relative overflow-hidden bg-surface px-9">
      {/* Green flowing light */}
      {running && (
        <div className="absolute inset-x-0 top-0 h-[2px]">
          <div className="h-full w-1/3 animate-[flowRight_2s_ease-in-out_infinite] rounded-full bg-gradient-to-r from-transparent via-running-dot to-transparent" />
        </div>
      )}
      <div className="flex items-center justify-between py-0.5">
        <div className="flex items-center gap-1.5">
          {running && (
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-running-dot" />
          )}
          <span className={`text-[10px] font-semibold ${running ? "animate-pulse text-running-dot" : "text-fg-muted"}`}>
            {stageName}
          </span>
        </div>
        <span className="text-[10px] font-mono text-fg-faint">
          Total: {elapsed}
        </span>
      </div>
    </div>
  );
}
