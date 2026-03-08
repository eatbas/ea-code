import type { ReactNode } from "react";
import type { PipelineRun } from "../../types";
import { formatDuration, formatTokens, parseCliResult } from "../../utils/formatters";
import { STAGE_LABELS } from "./constants";

interface ResultCardProps {
  run: PipelineRun;
  artifacts: Record<string, string>;
}

/** Final result card shown when a pipeline run completes. Green/red tinted. */
export function ResultCard({ run, artifacts }: ResultCardProps): ReactNode {
  const isOk = run.status === "completed";
  const tint = isOk ? "rgba(40,180,95,0.10)" : "rgba(230,75,75,0.10)";
  const borderTint = isOk ? "rgba(40,180,95,0.30)" : "rgba(230,75,75,0.30)";
  const verdictColour = isOk ? "#22c55e" : "#ef4444";

  // Try to extract structured data from result artifact (direct task CLI JSON)
  const cliResult = artifacts["result"] ? parseCliResult(artifacts["result"]) : null;

  // Result text: CLI result → executive_summary → fallback
  const resultText = cliResult?.result
    ?? artifacts["executive_summary"]
    ?? undefined;

  // Metrics
  const totalDuration = cliResult?.durationMs ?? computeDuration(run);
  const inputTokens = cliResult?.usage?.inputTokens;
  const outputTokens = cliResult?.usage?.outputTokens;
  const cacheReadTokens = cliResult?.usage?.cacheReadInputTokens;

  const totalInputDisplay = (inputTokens ?? 0) + (cacheReadTokens ?? 0);
  const hasTokens = inputTokens !== undefined || outputTokens !== undefined;

  // Stage breakdown rows
  const stageRows = run.iterations.flatMap((iter) =>
    iter.stages.filter((s) => s.durationMs > 0).map((s) => ({
      name: STAGE_LABELS[s.stage] ?? s.stage,
      durationMs: s.durationMs,
    })),
  );

  return (
    <div
      className="rounded-lg border p-3"
      style={{ background: tint, borderColor: borderTint }}
    >
      {/* Verdict */}
      <div className="mb-2 text-xs font-bold" style={{ color: verdictColour }}>
        {run.finalVerdict === "COMPLETE" ? "COMPLETE" : run.finalVerdict === "NOT COMPLETE" ? "NOT COMPLETE" : run.status.toUpperCase()}
      </div>

      {/* Result text */}
      {resultText && (
        <p className="mb-3 text-sm leading-relaxed text-[#e4e4ed] whitespace-pre-wrap">
          {resultText}
        </p>
      )}

      {/* Metrics */}
      <div className="flex flex-wrap items-center gap-3 border-t border-[#2e2e48]/50 pt-2 text-[11px]">
        {run.currentIteration > 0 && (
          <span className="text-[#9898b0]">
            <strong>Iterations:</strong> {run.currentIteration}
          </span>
        )}
        {totalDuration !== undefined && totalDuration > 0 && (
          <span className="text-[#9898b0]">
            <strong>Time:</strong> {formatDuration(totalDuration)}
          </span>
        )}
      </div>

      {/* Token display */}
      {hasTokens && (
        <div className="mt-1 flex items-center gap-3 text-[10px]">
          {totalInputDisplay > 0 && (
            <span className="text-blue-400">↑~{formatTokens(totalInputDisplay)}</span>
          )}
          {outputTokens !== undefined && outputTokens > 0 && (
            <span className="text-green-400">↓~{formatTokens(outputTokens)}</span>
          )}
        </div>
      )}

      {/* Cost breakdown table */}
      {stageRows.length > 0 && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Cost breakdown
          </summary>
          <table className="mt-1.5 w-full text-[10px]">
            <thead>
              <tr className="text-left text-[9px] font-semibold uppercase tracking-widest text-[#9898b0] opacity-70">
                <th className="py-0.5 pr-2">Step</th>
                <th className="py-0.5 text-right">Time</th>
              </tr>
            </thead>
            <tbody>
              {stageRows.map((row, i) => (
                <tr key={i} className="border-t border-[#2e2e48]/30 text-[#c4c4d4]">
                  <td className="py-0.5 pr-2">{row.name}</td>
                  <td className="py-0.5 text-right tabular-nums">{formatDuration(row.durationMs)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </details>
      )}

      {/* Raw output */}
      {artifacts["result"] && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Raw output
          </summary>
          <pre className="mt-1.5 max-h-48 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {artifacts["result"]}
          </pre>
        </details>
      )}

      {/* Judge details */}
      {artifacts["judge"] && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Judge details
          </summary>
          <pre className="mt-1.5 max-h-48 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {artifacts["judge"]}
          </pre>
        </details>
      )}
    </div>
  );
}

/** Computes duration from startedAt/completedAt ISO strings. */
function computeDuration(run: PipelineRun): number | undefined {
  if (!run.startedAt || !run.completedAt) return undefined;
  const start = new Date(run.startedAt).getTime();
  const end = new Date(run.completedAt).getTime();
  if (isNaN(start) || isNaN(end)) return undefined;
  return end - start;
}
