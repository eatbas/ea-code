import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { RefreshCw, RotateCcw, Square } from "lucide-react";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";

interface PipelineStatusBarProps {
  /** Name of the currently active stage (e.g. "Planner 1", "Code Fix"). */
  stageName: string;
  /** Whether the pipeline is actively running. */
  running: boolean;
  /** Epoch ms when the pipeline started. */
  startedAt: number;
  /** Epoch ms when the pipeline finished (stops the total timer). */
  finishedAt?: number;
  /** Whether the pipeline can be resumed (all stages terminal, not running). */
  canResume?: boolean;
  /** Whether any stages failed. */
  hasFailed?: boolean;
  /** Called when the user clicks Resume / Retry. */
  onResume?: () => void;
  /** Called when the user clicks Stop. */
  onStop?: () => void;
  /** Whether the user can re-do the review cycle. */
  canRedoReview?: boolean;
  /** Called when the user clicks Re-do Review. */
  onRedoReview?: () => void;
  /** Plan review phase. */
  reviewPhase?: PlanReviewPhase;
}

function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes > 0) {
    return `${String(minutes)}m ${String(seconds).padStart(2, "0")}s`;
  }
  return `${String(seconds)}s`;
}

export function PipelineStatusBar({
  stageName,
  running,
  startedAt,
  finishedAt,
  canResume,
  hasFailed,
  onResume,
  onStop,
  canRedoReview,
  onRedoReview,
  reviewPhase,
}: PipelineStatusBarProps): ReactNode {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, [running]);

  const elapsed = formatElapsed((finishedAt ?? now) - startedAt);
  const isReviewing = reviewPhase === "reviewing";
  const isSubmittingEdit = reviewPhase === "submitting_edit";

  // Centre content for the status bar.
  // Running state always takes priority so the Stop button and green
  // light are visible when the merge re-runs after feedback.
  const centreContent = (): ReactNode => {
    if (running) {
      return (
        <div className="flex items-center gap-2.5">
          {onStop && (
            <button
              type="button"
              onClick={onStop}
              className="inline-flex items-center gap-2 rounded-lg border border-error-border bg-error-bg px-4 py-1.5 text-xs font-semibold text-error-text transition-colors hover:opacity-80"
            >
              <Square size={10} fill="currentColor" />
              Stop Pipeline
            </button>
          )}
        </div>
      );
    }

    if (isReviewing) {
      return (
        <span className="text-xs font-semibold text-fg-muted">
          Review Plan
        </span>
      );
    }

    if (isSubmittingEdit) {
      return (
        <span className="text-xs font-semibold animate-pulse text-fg-muted">
          Updating plan...
        </span>
      );
    }

    // Default: Resume and Re-do Review buttons.
    return (
      <div className="flex items-center gap-2.5">
        {canRedoReview && onRedoReview && (
          <button
            type="button"
            onClick={onRedoReview}
            className="inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-4 py-1.5 text-xs font-semibold text-fg transition-colors hover:bg-active"
          >
            <RefreshCw size={10} />
            Re-do Review
          </button>
        )}
        {canResume && onResume && (
          <button
            type="button"
            onClick={onResume}
            className="inline-flex items-center gap-2 rounded-lg border border-edge bg-elevated px-4 py-1.5 text-xs font-semibold text-fg transition-colors hover:bg-active"
          >
            <RotateCcw size={10} />
            {hasFailed ? "Retry Pipeline" : "Resume Pipeline"}
          </button>
        )}
      </div>
    );
  };

  return (
    <div className="relative overflow-hidden bg-surface px-9">
      {/* Green flowing light */}
      {running && (
        <div className="absolute inset-x-0 top-0 h-[2px]">
          <div className="h-full w-1/3 animate-[flowRight_2s_ease-in-out_infinite] rounded-full bg-gradient-to-r from-transparent via-running-dot to-transparent" />
        </div>
      )}
      <div className="flex items-center justify-between py-2">
        <div className="flex items-center gap-1.5">
          {running && (
            <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-running-dot" />
          )}
          <span className={`text-xs font-semibold ${running ? "animate-pulse text-running-dot" : "text-fg-muted"}`}>
            {stageName}
          </span>
        </div>
        {centreContent()}
        <span className="text-xs font-mono text-fg-faint">
          Total: {elapsed}
        </span>
      </div>
    </div>
  );
}
