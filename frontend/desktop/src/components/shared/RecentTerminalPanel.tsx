import { useMemo, useState } from "react";
import type { ReactNode, RefObject } from "react";
import { useStickyAutoScroll } from "../../hooks/useStickyAutoScroll";

interface TerminalTab {
  label: string;
  lines: string[];
  totalLines?: number;
}

interface RecentTerminalPanelProps {
  label?: string;
  lines: string[];
  terminalRef: RefObject<HTMLPreElement | null>;
  onTerminalScroll?: () => void;
  /** Optional parallel terminal tabs (e.g., Plan 1, Plan 2, Plan 3). */
  parallelTabs?: TerminalTab[];
}

export function RecentTerminalPanel({
  label,
  lines,
  terminalRef,
  onTerminalScroll,
  parallelTabs,
}: RecentTerminalPanelProps): ReactNode {
  const [activeTabIdx, setActiveTabIdx] = useState(0);

  const showTabs = parallelTabs && parallelTabs.length > 1;
  const activeTab = showTabs ? parallelTabs[activeTabIdx] : null;
  const displayLines = activeTab ? activeTab.lines : lines;
  const parallelDependencyKey = useMemo(
    () => `${activeTab?.label ?? "none"}:${activeTab?.totalLines ?? activeTab?.lines.length ?? 0}`,
    [activeTab?.label, activeTab?.lines.length, activeTab?.totalLines],
  );
  const { scrollRef: parallelRef, onScroll: onParallelScroll } = useStickyAutoScroll<HTMLPreElement>(parallelDependencyKey);
  const displayRef = activeTab ? parallelRef : terminalRef;
  const handleScroll = activeTab ? onParallelScroll : onTerminalScroll;

  return (
    <details className="w-full rounded-xl border border-[#2e2e48] bg-[#14141e]">
      <summary className="cursor-pointer select-none px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-[#9898b0] hover:text-[#e4e4ed] transition-colors">
        Recent Terminal{label && !showTabs ? ` - ${label}` : ""}
        {showTabs ? ` - ${parallelTabs.length} planners` : ""}
      </summary>
      <div className="border-t border-[#2e2e48] p-3">
        {/* Parallel tabs */}
        {showTabs && (
          <div className="mb-2 flex gap-1">
            {parallelTabs.map((tab, idx) => (
              <button
                key={tab.label}
                type="button"
                onClick={() => setActiveTabIdx(idx)}
                className={`rounded px-2 py-1 text-[10px] font-medium transition-colors ${
                  idx === activeTabIdx
                    ? "bg-[#40c4ff]/15 text-[#40c4ff]"
                    : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
                }`}
              >
                {tab.label}
              </button>
            ))}
          </div>
        )}
        <pre
          ref={displayRef}
          onScroll={handleScroll}
          className="app-scrollbar max-h-56 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] leading-relaxed text-[#e4e4ed] whitespace-pre-wrap break-words"
        >
          {displayLines.length > 0 ? displayLines.join("\n") : "Waiting for terminal output..."}
        </pre>
      </div>
    </details>
  );
}
