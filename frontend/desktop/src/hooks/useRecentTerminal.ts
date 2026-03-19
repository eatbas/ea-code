import { useMemo } from "react";
import type { RefObject } from "react";
import { useStickyAutoScroll } from "./useStickyAutoScroll";

interface RecentTerminalResult {
  terminalRef: RefObject<HTMLPreElement | null>;
  onTerminalScroll: () => void;
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
  const stage = currentStage
    ?? [...allStages]
      .reverse()
      .map((s) => s.stage)
      .find((s) => (stageLogs[s]?.length ?? 0) > 0);

  const lines = stage ? (stageLogs[stage] ?? []).slice(-maxLines) : [];
  const label = stage?.replace(/_/g, " ");
  const totalLines = stage ? (stageLogs[stage]?.length ?? 0) : 0;
  const dependencyKey = useMemo(
    () => `${stage ?? "none"}:${totalLines}`,
    [stage, totalLines],
  );
  const { scrollRef: terminalRef, onScroll: onTerminalScroll } = useStickyAutoScroll<HTMLPreElement>(dependencyKey);

  return { terminalRef, onTerminalScroll, stage, lines, label };
}
