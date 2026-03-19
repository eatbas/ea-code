import type { ReactNode } from "react";
import type { PipelineStage } from "../../types";
import { formatDuration, formatTimestamp, normaliseDisplayText, parseUtcTimestamp } from "../../utils/formatters";
import { statusToneClasses } from "../../utils/statusHelpers";
import { stageLabel } from "./constants";

interface ResultCardProps {
  /** Run status - "completed", "failed", "cancelled", etc. */
  status: string;
  finalVerdict?: string;
  iterationCount: number;
  totalDurationMs?: number;
  completedAt?: string;
  executiveSummary?: string;
  error?: string;
  /** Stage timing rows for collapsible breakdown. */
  stageRows?: { name: string; durationMs: number }[];
  /** Optional judge reasoning to display. */
  judgeReasoning?: string;
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
  judgeReasoning,
}: ResultCardProps): ReactNode {
  const statusClasses = statusToneClasses(status);
  const unresolvedRequired = finalVerdict === "NOT COMPLETE"
    ? extractJudgeSection(judgeReasoning, "Checklist")
        ?.split("\n")
        .filter((line) => line.includes("[ ]") && line.includes("[REQUIRED]"))
        .join("\n")
    : null;
  const testAssessment = extractJudgeSection(judgeReasoning, "Test Assessment");
  const nextSteps = finalVerdict === "NOT COMPLETE"
    ? extractJudgeSection(judgeReasoning, "Next Steps")
    : null;

  return (
    <div className={`rounded-lg border px-3 py-2 ${statusClasses.cardBg} ${statusClasses.cardBorder}`}>
      {/* Status row */}
      <div className="flex items-center gap-2">
        <div className={`h-2 w-2 rounded-full ${statusClasses.dot}`} />
        <span className={`text-xs font-medium capitalize ${statusClasses.text}`}>
          {status}
        </span>
        {finalVerdict && (
          <span className={`rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase ${statusClasses.badge}`}>
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
      {executiveSummary && (
        <p className="mt-2 whitespace-pre-wrap text-xs leading-relaxed text-[#c4c4d4]">
          {executiveSummary}
        </p>
      )}
      {error && (
        <p className="mt-1.5 text-xs text-[#ef4444]">{error}</p>
      )}
      {unresolvedRequired && (
        <div className="mt-2 rounded border border-amber-400/20 bg-amber-400/5 p-2">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-amber-300">
            Unresolved Required Items
          </p>
          <pre className="mt-1 whitespace-pre-wrap break-words text-[11px] text-[#e4e4ed]">
            {normaliseDisplayText(unresolvedRequired)}
          </pre>
        </div>
      )}
      {testAssessment && (
        <div className="mt-2 rounded border border-[#2e2e48] bg-[#0f0f14] p-2">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-[#9898b0]">
            Test Assessment
          </p>
          <pre className="mt-1 whitespace-pre-wrap break-words text-[11px] text-[#c4c4d4]">
            {normaliseDisplayText(testAssessment)}
          </pre>
        </div>
      )}
      {nextSteps && (
        <div className="mt-2 rounded border border-rose-400/20 bg-rose-400/5 p-2">
          <p className="text-[10px] font-semibold uppercase tracking-wider text-rose-300">
            Next Steps
          </p>
          <pre className="mt-1 whitespace-pre-wrap break-words text-[11px] text-[#e4e4ed]">
            {normaliseDisplayText(nextSteps)}
          </pre>
        </div>
      )}

      {/* Judge reasoning */}
      {judgeReasoning && (
        <details className="mt-2">
          <summary className="cursor-pointer text-[10px] text-[#9898b0] opacity-70">
            Judge details
          </summary>
          <pre className="mt-1.5 overflow-x-auto rounded bg-[#0f0f14] p-2 text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
            {normaliseDisplayText(judgeReasoning)}
          </pre>
        </details>
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
    </div>
  );
}

/** Builds stage timing rows from events array.
 *  Extracts stage timing from RunEvent timeline.
 */
export function buildStageRowsFromEvents(
  events: { stage?: string; durationMs?: number; type?: string }[],
): { name: string; durationMs: number }[] {
  const stageDurations = new Map<string, number>();

  for (const event of events) {
    if (event.type === "stage_end" && event.stage && event.durationMs) {
      const existing = stageDurations.get(event.stage) ?? 0;
      stageDurations.set(event.stage, existing + event.durationMs);
    }
  }

  return Array.from(stageDurations.entries())
    .filter(([, durationMs]) => durationMs > 0)
    .map(([stage, durationMs]) => ({
      name: stageLabel(stage as PipelineStage),
      durationMs,
    }));
}

/** Builds stage timing rows from a stages array (for legacy/live pipeline use).
 *  Works with both StageResult and old StageEntry shapes.
 */
export function buildStageRows(
  stages: { stage: string; durationMs: number }[],
): { name: string; durationMs: number }[] {
  return stages
    .filter((s) => s.durationMs > 0)
    .map((s) => ({
      name: stageLabel(s.stage as PipelineStage),
      durationMs: s.durationMs,
    }));
}

/** Computes duration from startedAt/completedAt timestamp strings.
 *  Uses parseUtcTimestamp to handle bare SQLite timestamps correctly. */
export function computeDuration(startedAt?: string, completedAt?: string): number | undefined {
  if (!startedAt || !completedAt) return undefined;
  const start = parseUtcTimestamp(startedAt).getTime();
  const end = parseUtcTimestamp(completedAt).getTime();
  if (isNaN(start) || isNaN(end)) return undefined;
  return end - start;
}

function extractJudgeSection(text: string | undefined, heading: string): string | null {
  if (!text) return null;
  const marker = `## ${heading}`;
  const start = text.indexOf(marker);
  if (start === -1) return null;
  const after = text.slice(start + marker.length);
  const next = after.search(/\n##\s+/);
  return (next === -1 ? after : after.slice(0, next)).trim() || null;
}
