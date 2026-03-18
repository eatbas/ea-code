import type { ReactNode } from "react";
import { useMemo } from "react";
import { useStickyAutoScroll } from "../hooks/useStickyAutoScroll";

interface LogsPanelProps {
  logs: string[];
}

/** Scrollable monospace log output panel with auto-scroll behaviour. */
export function LogsPanel({ logs }: LogsPanelProps): ReactNode {
  const dependencyKey = useMemo(() => logs.length, [logs.length]);
  const { scrollRef: containerRef, onScroll } = useStickyAutoScroll<HTMLDivElement>(dependencyKey);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b border-[#2e2e48]">
        <span className="text-xs font-medium text-[#9898b0]">Logs</span>
      </div>
      <div
        ref={containerRef}
        onScroll={onScroll}
        className="app-scrollbar bg-[#0f0f14] font-mono text-xs text-[#e4e4ed] overflow-y-auto flex-1 p-3"
      >
        {logs.length === 0 ? (
          <span className="text-[#9898b0]">No logs yet.</span>
        ) : (
          logs.map((line, idx) => (
            <div key={idx} className="whitespace-pre-wrap break-all leading-5">
              {line}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
