import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { ChevronDown } from "lucide-react";

export type StageStatus = "pending" | "running" | "completed" | "failed" | "stopped";

interface PipelineStageSectionProps {
  /** Stage label shown in the header (e.g. "Planner 1", "Coder"). */
  label: string;
  /** Agent display name (e.g. "Claude / Opus"). */
  agentLabel?: string;
  status: StageStatus;
  /** Whether the section starts expanded (uncontrolled mode). */
  defaultOpen?: boolean;
  /** Controlled open state — overrides internal state when provided. */
  open?: boolean;
  /** Called when the user toggles the section (controlled mode). */
  onOpenChange?: (open: boolean) => void;
  /** Epoch ms when the stage started (enables timer). */
  startedAt?: number;
  /** Epoch ms when the stage finished (stops timer). */
  finishedAt?: number;
  children: ReactNode;
}

const STATUS_STYLES: Record<StageStatus, { dot: string; label: string; text: string }> = {
  pending: { dot: "bg-fg-faint", label: "Pending", text: "text-fg-faint" },
  running: { dot: "bg-running-dot animate-pulse", label: "Running", text: "text-fg" },
  completed: { dot: "hidden", label: "Done", text: "text-success-chip-text" },
  failed: { dot: "bg-error-text", label: "Failed", text: "text-error-text" },
  stopped: { dot: "bg-warning-text", label: "Stopped", text: "text-warning-text" },
};

function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${String(minutes)}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${String(seconds)}s`;
}

export function PipelineStageSection({
  label,
  agentLabel,
  status,
  defaultOpen,
  open: controlledOpen,
  onOpenChange,
  startedAt,
  finishedAt,
  children,
}: PipelineStageSectionProps): ReactNode {
  const [internalOpen, setInternalOpen] = useState(defaultOpen ?? status === "running");
  const [now, setNow] = useState(Date.now());
  const isOpen = controlledOpen ?? internalOpen;

  // Tick every second while running.
  useEffect(() => {
    if (status !== "running" || !startedAt) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [status, startedAt]);

  function toggle(): void {
    const next = !isOpen;
    if (controlledOpen === undefined) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
  }

  const style = STATUS_STYLES[status];

  const elapsed = startedAt
    ? formatElapsed((finishedAt ?? now) - startedAt)
    : null;

  return (
    <div className="flex flex-col rounded-xl border border-edge bg-panel">
      <button
        type="button"
        onClick={toggle}
        className="flex w-full items-center gap-3 px-4 py-3 text-left"
      >
        <span className={`h-2 w-2 shrink-0 rounded-full ${style.dot}`} />
        <span className="text-xs font-semibold text-fg">{label}</span>
        {agentLabel && (
          <span className="text-[10px] text-fg-muted">{agentLabel}</span>
        )}
        <span className="ml-auto flex items-center gap-3">
          {elapsed && (
            <span className="text-[10px] font-mono text-fg-faint">{elapsed}</span>
          )}
          <span className={`text-[10px] font-medium uppercase tracking-wider ${style.text}`}>
            {style.label}
          </span>
        </span>
        <ChevronDown
          size={14}
          className={`shrink-0 text-fg-muted transition-transform ${isOpen ? "rotate-180" : ""}`}
        />
      </button>
      {isOpen && (
        <div className="min-h-0 flex-1 max-h-48 overflow-y-auto border-t border-edge px-4 py-3 pipeline-scroll">
          {children}
        </div>
      )}
    </div>
  );
}
