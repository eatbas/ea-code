import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import type { PipelineStageState } from "../../hooks/usePipelineSession";
import { type StageStatus, formatElapsed, STATUS_STYLES } from "./PipelineStageSection";

interface PipelineStageGroupProps {
  /** Group label shown in the header (e.g. "Planners", "Reviewers"). */
  groupLabel: string;
  /** Stages in this group — used to derive max elapsed time and aggregate status. */
  stages: PipelineStageState[];
  /** The grid of individual stage cards. */
  children: ReactNode;
}

function computeGroupStatus(stages: PipelineStageState[]): StageStatus {
  if (stages.length === 0) return "pending";
  if (stages.some((s) => s.status === "running")) return "running";
  if (stages.some((s) => s.status === "failed")) return "failed";
  if (stages.some((s) => s.status === "stopped")) return "stopped";
  if (stages.every((s) => s.status === "completed")) return "completed";
  return "pending";
}

function computeMaxElapsed(stages: PipelineStageState[], now: number): number | null {
  let max = 0;
  let hasAny = false;
  for (const stage of stages) {
    if (stage.startedAt != null) {
      hasAny = true;
      const elapsed = (stage.finishedAt ?? now) - stage.startedAt;
      if (elapsed > max) max = elapsed;
    }
  }
  return hasAny ? max : null;
}

export function PipelineStageGroup({
  groupLabel,
  stages,
  children,
}: PipelineStageGroupProps): ReactNode {
  const [now, setNow] = useState(Date.now());
  const groupStatus = computeGroupStatus(stages);

  useEffect(() => {
    if (groupStatus !== "running") return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [groupStatus]);

  const maxElapsed = computeMaxElapsed(stages, now);
  const style = STATUS_STYLES[groupStatus];

  return (
    <div className="rounded-2xl border border-edge p-3">
      <div className="mb-2 flex items-center justify-between px-1">
        <span className="text-[11px] font-semibold uppercase tracking-[0.12em] text-fg-muted">
          {groupLabel}
        </span>
        <span className="flex items-center gap-2">
          {maxElapsed != null && (
            <span className="text-[10px] font-mono text-fg-faint">
              {formatElapsed(maxElapsed)}
            </span>
          )}
          <span className={`h-2 w-2 shrink-0 rounded-full ${style.dot}`} />
          <span className={`text-[10px] font-medium uppercase tracking-wider ${style.text}`}>
            {style.label}
          </span>
        </span>
      </div>
      {children}
    </div>
  );
}
