import { useEffect, useRef } from "react";

interface RecentTerminalResult {
  terminalRef: React.RefObject<HTMLPreElement | null>;
  stage: string | undefined;
  lines: string[];
  label: string | undefined;
}

/**
 * Derives the most relevant terminal stage and its recent output lines,
 * plus an auto-scrolling ref for the terminal container.
 */
export function useRecentTerminal(
  stageLogs: Record<string, string[]>,
  currentStage: string | undefined,
  allStages: { stage: string }[],
  maxLines = 160,
): RecentTerminalResult {
  const terminalRef = useRef<HTMLPreElement>(null);

  const stage = currentStage
    ?? [...allStages]
      .reverse()
      .map((s) => s.stage)
      .find((s) => (stageLogs[s]?.length ?? 0) > 0);

  const lines = stage ? (stageLogs[stage] ?? []).slice(-maxLines) : [];
  const label = stage?.replace(/_/g, " ");

  useEffect(() => {
    const el = terminalRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [stage, lines.length]);

  return { terminalRef, stage, lines, label };
}
