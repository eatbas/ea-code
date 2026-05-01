import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { ChevronDown, RotateCcw } from "lucide-react";

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
  /**
   * If provided and the stage is `failed`, a Retry button is shown
   * in the header. Clicking sends a `continue` turn to the captured
   * provider session — distinct from the pipeline-wide retry that
   * re-issues the original prompt.
   */
  onRetry?: () => void;
  /** Whether a retry is currently in flight (disables the button). */
  retryPending?: boolean;
  children: ReactNode;
}

export const STATUS_STYLES: Record<StageStatus, { dot: string; label: string; text: string }> = {
  pending: { dot: "bg-fg-faint", label: "Pending", text: "text-fg-faint" },
  running: { dot: "bg-running-dot animate-pulse", label: "Running", text: "text-fg" },
  completed: { dot: "hidden", label: "Done", text: "text-success-chip-text" },
  failed: { dot: "bg-error-text", label: "Failed", text: "text-error-text" },
  stopped: { dot: "bg-warning-text", label: "Stopped", text: "text-warning-text" },
};

export function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(Math.max(0, ms) / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${String(minutes)}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${String(seconds)}s`;
}

/** Shorten parallel stage labels: "Planner 1" → "P1", "Reviewer 2" → "R2". */
function shortLabel(label: string): string {
  const m = label.match(/^(Planner|Reviewer)\s+(\d+)$/);
  if (m) return `${m[1][0]}${m[2]}`;
  return label;
}

/** Strip the "provider / " prefix so only the model name is shown. */
function modelOnly(raw: string): string {
  const idx = raw.lastIndexOf("/");
  return idx === -1 ? raw : raw.slice(idx + 1).trim();
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
  onRetry,
  retryPending,
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

  // The header is a div + role="button" rather than a real <button> so
  // we can nest a real Retry <button> inside without producing invalid
  // HTML (button-in-button) when a stage fails.
  return (
    <div className="flex flex-col rounded-xl border border-edge bg-panel min-w-0">
      <div
        role="button"
        tabIndex={0}
        aria-expanded={isOpen}
        onClick={toggle}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            toggle();
          }
        }}
        className="flex w-full cursor-pointer items-center gap-2 px-4 py-3 text-left min-w-0 whitespace-nowrap"
      >
        <span className={`h-2 w-2 shrink-0 rounded-full ${style.dot}`} />
        <span className="shrink-0 text-xs font-semibold text-fg">{shortLabel(label)}</span>
        {agentLabel && (
          <span className="truncate text-[10px] text-fg-muted">{modelOnly(agentLabel)}</span>
        )}
        <span className="ml-auto flex shrink-0 items-center gap-2">
          {elapsed && (
            <span className="text-[10px] font-mono text-fg-faint">{elapsed}</span>
          )}
          {status === "failed" && onRetry && (
            <button
              type="button"
              aria-label="Retry stage with continue"
              title="Resume this stage by sending `continue` to its session"
              disabled={retryPending}
              onClick={(event) => {
                event.stopPropagation();
                onRetry();
              }}
              className="inline-flex items-center gap-1 rounded-md border border-edge bg-elevated px-2 py-0.5 text-[10px] font-semibold text-fg transition-colors hover:bg-active disabled:cursor-not-allowed disabled:opacity-60"
            >
              <RotateCcw size={10} className={retryPending ? "animate-spin" : ""} />
              {retryPending ? "Retrying..." : "Retry"}
            </button>
          )}
          <span className={`text-[10px] font-medium uppercase tracking-wider ${style.text}`}>
            {style.label}
          </span>
        </span>
        <ChevronDown
          size={14}
          className={`shrink-0 text-fg-muted transition-transform ${isOpen ? "rotate-180" : ""}`}
        />
      </div>
      {isOpen && (
        <div className="min-h-0 flex-1 max-h-48 overflow-y-auto overflow-x-hidden border-t border-edge px-4 py-3 pipeline-scroll">
          {children}
        </div>
      )}
    </div>
  );
}
