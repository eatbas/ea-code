import type { ReactNode } from "react";
import type { RunDetail } from "../types";
import { formatDuration } from "../utils/formatters";

interface RunCardProps {
  run: RunDetail;
}

/** Displays a single historical run as a prompt bubble + result card. */
export function RunCard({ run }: RunCardProps): ReactNode {
  const verdictColour = run.finalVerdict === "pass"
    ? "#22c55e"
    : run.finalVerdict === "fail"
      ? "#ef4444"
      : "#9898b0";

  const statusColour = run.status === "completed"
    ? "#22c55e"
    : run.status === "failed"
      ? "#ef4444"
      : run.status === "cancelled"
        ? "#f59e0b"
        : "#9898b0";

  const isOk = run.status === "completed";
  const tint = isOk ? "rgba(40,180,95,0.10)" : run.status === "failed" ? "rgba(230,75,75,0.10)" : undefined;
  const borderTint = isOk ? "rgba(40,180,95,0.30)" : run.status === "failed" ? "rgba(230,75,75,0.30)" : "#2e2e48";

  // Compute duration from run timestamps
  const totalDurationMs = run.startedAt && run.completedAt
    ? new Date(run.completedAt).getTime() - new Date(run.startedAt).getTime()
    : 0;

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt — right-aligned bubble */}
      <div className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
          {run.prompt}
        </div>
      </div>

      {/* Run result — left-aligned */}
      <div className="flex justify-start">
        <div
          className="w-full rounded-2xl rounded-bl-md border px-4 py-3"
          style={{ background: tint ?? "#1a1a24", borderColor: borderTint }}
        >
          {/* Status row */}
          <div className="flex items-center gap-2 mb-2">
            <div className="h-2 w-2 rounded-full" style={{ backgroundColor: statusColour }} />
            <span className="text-xs font-medium capitalize" style={{ color: statusColour }}>
              {run.status}
            </span>
            {run.finalVerdict && (
              <span className="text-xs px-1.5 py-0.5 rounded" style={{ color: verdictColour, backgroundColor: `${verdictColour}15` }}>
                {run.finalVerdict}
              </span>
            )}
            {run.completedAt && (
              <span className="ml-auto text-xs text-[#6f7086]">
                {formatTimestamp(run.completedAt)}
              </span>
            )}
          </div>

          {/* Executive summary */}
          {run.executiveSummary && (
            <p className="text-sm text-[#c4c4d4] whitespace-pre-wrap leading-relaxed">
              {run.executiveSummary}
            </p>
          )}

          {/* Error message */}
          {run.error && (
            <p className="text-xs text-[#ef4444] mt-2">
              {run.error}
            </p>
          )}

          {/* Metrics row */}
          <div className="mt-2 flex items-center gap-3 text-[11px] text-[#6f7086]">
            {run.iterations.length > 0 && (
              <span>{run.iterations.length} {run.iterations.length === 1 ? "iteration" : "iterations"}</span>
            )}
            {totalDurationMs > 0 && (
              <span>{formatDuration(totalDurationMs)}</span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** Formats an ISO timestamp into a readable date/time string. */
function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) +
      " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}
