import type { ReactNode } from "react";
import { useState, useEffect } from "react";
import type { PipelineStage } from "../../types";
import { STAGE_LABELS } from "./constants";

interface ThinkingIndicatorProps {
  stage: PipelineStage;
}

/** Animated sweep indicator shown for the currently running pipeline stage. */
export function ThinkingIndicator({ stage }: ThinkingIndicatorProps): ReactNode {
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    setElapsed(0);
    const interval = setInterval(() => setElapsed((prev) => prev + 1), 1000);
    return () => clearInterval(interval);
  }, [stage]);

  const mins = Math.floor(elapsed / 60);
  const secs = elapsed % 60;
  const timer = mins > 0 ? `${mins}:${String(secs).padStart(2, "0")}` : `0:${String(secs).padStart(2, "0")}`;

  return (
    <div className="relative overflow-hidden rounded-lg border border-[#2e2e48] bg-[#161622] px-3 py-2">
      {/* Sweep overlay */}
      <div
        className="pointer-events-none absolute inset-0"
        style={{
          background: "linear-gradient(90deg, transparent 0%, rgba(34,197,94,0.08) 35%, rgba(34,197,94,0.34) 50%, rgba(34,197,94,0.08) 65%, transparent 100%)",
          animation: "thinking-sweep 1.6s linear infinite",
        }}
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
