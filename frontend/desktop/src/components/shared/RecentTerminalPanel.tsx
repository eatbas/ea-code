import type { ReactNode, RefObject } from "react";

interface RecentTerminalPanelProps {
  label?: string;
  lines: string[];
  terminalRef: RefObject<HTMLPreElement | null>;
  onTerminalScroll?: () => void;
}

export function RecentTerminalPanel({
  label,
  lines,
  terminalRef,
  onTerminalScroll,
}: RecentTerminalPanelProps): ReactNode {
  return (
    <details className="w-full rounded-xl border border-[#2e2e48] bg-[#14141e]">
      <summary className="cursor-pointer select-none px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-[#9898b0] hover:text-[#e4e4ed] transition-colors">
        Recent Terminal{label ? ` - ${label}` : ""}
      </summary>
      <div className="border-t border-[#2e2e48] p-3">
        <pre
          ref={terminalRef}
          onScroll={onTerminalScroll}
          className="max-h-56 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] leading-relaxed text-[#e4e4ed] whitespace-pre-wrap break-words"
        >
          {lines.length > 0 ? lines.join("\n") : "Waiting for terminal output..."}
        </pre>
      </div>
    </details>
  );
}
