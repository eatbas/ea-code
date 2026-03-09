import type { ReactNode } from "react";
import type { PipelineStage } from "../../types";
import { formatDuration, formatTimestamp, formatTokens, parseCliResult } from "../../utils/formatters";
import { STAGE_LABELS } from "./constants";

interface ResultCardProps {
  /** Run status — "completed", "failed", "cancelled", etc. */
  status: string;
  finalVerdict?: string;
  iterationCount: number;
  totalDurationMs?: number;
  completedAt?: string;
  executiveSummary?: string;
  error?: string;
  /** Stage timing rows for collapsible breakdown. */
  stageRows?: { name: string; durationMs: number }[];
  /** Raw artifact map — used for tokens, raw output, and judge detail sections. */
  artifacts?: Record<string, string>;
}

/** Unified result card used by both ChatView and RunCard (session history). */
export function ResultCard({
  status,
  finalVerdict,
  iterationCount,
  totalDurationMs,
  completedAt,
  executiveSummary,
  error,
  stageRows,
  artifacts,
}: ResultCardProps): ReactNode {
  const statusColour = status === "completed"
    ? "#22c55e"
    : status === "failed"
      ? "#ef4444"
      : status === "cancelled"
        ? "#f59e0b"
        : "#9898b0";

  const isOk = status === "completed";
  const bgTint = isOk ? "rgba(40,180,95,0.10)" : status === "failed" ? "rgba(230,75,75,0.10)" : "#1a1a24";
  const borderTint = isOk ? "rgba(40,180,95,0.30)" : status === "failed" ? "rgba(230,75,75,0.30)" : "#2e2e48";

  // Extract token info from CLI result artifact if available
  const cliResult = artifacts?.["result"] ? parseCliResult(artifacts["result"]) : null;
  const inputTokens = cliResult?.usage?.inputTokens;
  const outputTokens = cliResult?.usage?.outputTokens;
  const cacheReadTokens = cliResult?.usage?.cacheReadInputTokens;
  const totalInputDisplay = (inputTokens ?? 0) + (cacheReadTokens ?? 0);
  const hasTokens = inputTokens !== undefined || outputTokens !== undefined;

  // Use CLI result text or executive summary
  const resultText = cliResult?.result ?? executiveSummary;

  return (
    <div
      className="rounded-lg border px-3 py-2"
      style={{ background: bgTint, borderColor: borderTint }}
    >
      {/* Status row */}
      <div className="flex items-center gap-2">
        <div className="h-2 w-2 rounded-full" style={{ backgroundColor: statusColour }} />
        <span className="text-xs font-medium capitalize" style={{ color: statusColour }}>
          {status}
        </span>
        {finalVerdict && (
          <span
            className="text-[10px] px-1.5 py-0.5 rounded uppercase font-semibold"
            style={{ color: statusColour, backgroundColor: `${statusColour}15` }}
          >
            {finalVerdict}
          </span>
        )}
        <div className="ml-auto flex items-center gap-2 text-[11px] text-[#6f7086]">
          {iterationCount > 0 && (
            <span>{iterationCount} {iterationCount === 1 ? "iteration" : "iterations"}</span>
          )}
          {totalDurationMs != null && totalDurationMs > 0 && (
            <span>{formatDuration(totalDurationMs)}</span>
          )}
          {completedAt && (
            <span>{formatTimestamp(completedAt)}</span>
          )}
        </div>
      </div>

      {/* Result / executive summary text */}
      {resultText && (
        <p className="mt-2 text-xs text-[#c4c4d4] whitespace-pre-wrap leading-relaxed">
          {resultText}
        </p>
      )}
      {error && (
        <p className="mt-1.5 text-xs text-[#ef4444]">{error}</p>
      )}

      {/* Token display */}
      {hasTokens && (
        <div className="mt-1.5 flex items-center gap-3 text-[10px]">
          {totalInputDisplay > 0 && (
            <span className="text-blue-400">↑~{formatTokens(totalInputDisplay)}</span>
          )}
          {outputTokens !== undefined && outputTokens > 0 && (
            <span className="text-green-400">↓~{formatTokens(outputTokens)}</span>
          )}
        </div>
      )}

      {/* Collapsible: cost breakdown */}
      {stageRows && stageRows.length > 0 && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Stage breakdown
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

      {/* Collapsible: raw output */}
      {artifacts?.["result"] && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Raw output
          </summary>
          <pre className="mt-1.5 max-h-48 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {artifacts["result"]}
          </pre>
        </details>
      )}

      {/* Collapsible: judge details */}
      {artifacts?.["judge"] && (
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

/** Builds stage timing rows from a stages array. Works with both StageResult and StageEntry shapes. */
export function buildStageRows(
  stages: { stage: string; durationMs: number }[],
): { name: string; durationMs: number }[] {
  return stages
    .filter((s) => s.durationMs > 0)
    .map((s) => ({
      name: STAGE_LABELS[s.stage as PipelineStage] ?? s.stage,
      durationMs: s.durationMs,
    }));
}

/** Computes duration from startedAt/completedAt ISO strings. */
export function computeDuration(startedAt?: string, completedAt?: string): number | undefined {
  if (!startedAt || !completedAt) return undefined;
  const start = new Date(startedAt).getTime();
  const end = new Date(completedAt).getTime();
  if (isNaN(start) || isNaN(end)) return undefined;
  return end - start;
}
