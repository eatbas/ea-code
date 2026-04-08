import type { ReactNode } from "react";
import { useCallback, useState } from "react";
import { ChevronDown, Clipboard } from "lucide-react";

interface SidecarDebugConsoleProps {
  logs: string;
}

export function SidecarDebugConsole({ logs }: SidecarDebugConsoleProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    if (!logs.trim()) return;
    await navigator.clipboard.writeText(logs);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [logs]);

  return (
    <div className="rounded-2xl border border-edge bg-panel">
      <div className="flex items-center justify-between border-b border-edge px-4 py-3">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="flex min-w-0 flex-1 items-center gap-3 text-left"
          aria-expanded={open}
        >
          <div className="min-w-0 flex-1">
            <p className="text-[11px] font-medium uppercase tracking-[0.12em] text-fg-subtle">
              Sidecar Debug
            </p>
            <p className="text-xs text-fg-muted">
              Symphony stdout/stderr. Copy and send for diagnosis.
            </p>
          </div>
          <ChevronDown
            size={14}
            className={`shrink-0 text-fg-muted transition-transform ${open ? "rotate-180" : ""}`}
          />
        </button>
        <button
          type="button"
          onClick={() => { void handleCopy(); }}
          disabled={!logs.trim()}
          className="ml-3 inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-3 py-1.5 text-xs font-semibold text-fg transition-colors hover:bg-active disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Clipboard size={12} />
          {copied ? "Copied" : "Copy Log"}
        </button>
      </div>
      {open && (
        <div className="max-h-56 overflow-auto px-4 py-3 pipeline-scroll">
          <pre className="whitespace-pre-wrap break-words font-mono text-[11px] leading-5 text-fg-muted">
            {logs}
          </pre>
        </div>
      )}
    </div>
  );
}
