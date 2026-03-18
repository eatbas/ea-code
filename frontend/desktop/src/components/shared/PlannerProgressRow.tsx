import { useState, useEffect, useRef } from "react";
import type { ReactNode } from "react";
import type { StageResult, AppSettings } from "../../types";
import { formatDuration } from "../../utils/formatters";
import { stageModelLabel } from "../../utils/stageModelLabels";

const PLAN_INDEX_LABELS: Record<string, string> = {
  plan: "Plan 1",
  plan2: "Plan 2",
  plan3: "Plan 3",
};

interface PlannerProgressRowProps {
  stages: StageResult[];
  settings: AppSettings | null;
}

/** Shows a row of mini progress bars, one per planner. */
export function PlannerProgressRow({ stages, settings }: PlannerProgressRowProps): ReactNode {
  const [, tick] = useState(0);
  const anyRunning = stages.some((s) => s.status === "running");
  const fallbackStartTimes = useRef<Record<string, number>>({});

  for (const stage of stages) {
    if (stage.status !== "running") {
      continue;
    }

    if (stage.startedAt != null) {
      fallbackStartTimes.current[stage.stage] = stage.startedAt;
      continue;
    }

    if (fallbackStartTimes.current[stage.stage] == null) {
      fallbackStartTimes.current[stage.stage] = Date.now();
    }
  }

  useEffect(() => {
    if (!anyRunning) return;
    const interval = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [anyRunning]);

  return (
    <div className="grid gap-1.5" style={{ gridTemplateColumns: `repeat(${stages.length}, 1fr)` }}>
      {stages.map((stage) => {
        const label = PLAN_INDEX_LABELS[stage.stage] ?? stage.stage;
        const model = stageModelLabel(stage.stage, settings);
        const isRunning = stage.status === "running";
        const isCompleted = stage.status === "completed";
        const isFailed = stage.status === "failed";
        const resolvedStartedAt = stage.startedAt ?? fallbackStartTimes.current[stage.stage];

        const elapsed = isRunning && resolvedStartedAt != null
          ? Math.max(0, Math.floor((Date.now() - resolvedStartedAt) / 1000))
          : 0;
        const mins = Math.floor(elapsed / 60);
        const secs = elapsed % 60;
        const timer = `${mins}:${String(secs).padStart(2, "0")}`;

        return (
          <div
            key={stage.stage}
            className="relative overflow-hidden rounded-lg border border-[#2e2e48] bg-[#161622] px-2.5 py-1.5"
          >
            {isRunning && (
              <div className="thinking-sweep-overlay pointer-events-none absolute inset-0" />
            )}
            <div className="relative z-10 flex items-center gap-1.5">
              {/* Status icon */}
              {isCompleted && (
                <svg className="h-3 w-3 shrink-0 text-[#22c55e]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              )}
              {isFailed && (
                <svg className="h-3 w-3 shrink-0 text-[#ef4444]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              )}
              {isRunning && (
                <span className="relative flex h-2 w-2 shrink-0">
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#40c4ff] opacity-75" />
                  <span className="relative inline-flex h-2 w-2 rounded-full bg-[#40c4ff]" />
                </span>
              )}

              <span className={`text-[10px] font-semibold tracking-wide truncate ${
                isCompleted ? "text-[#22c55e]" : isRunning ? "text-[#4ade80]" : isFailed ? "text-[#ef4444]" : "text-[#9898b0]"
              }`}>
                {label}
              </span>
              {model && (
                <span className="hidden sm:inline text-[8px] text-[#9898b0] opacity-60 truncate">{model}</span>
              )}
              <span className="ml-auto text-[9px] tabular-nums text-[#9898b0] opacity-80 shrink-0">
                {isRunning ? timer : isCompleted || isFailed ? formatDuration(stage.durationMs) : ""}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
