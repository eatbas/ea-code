import type { ReactNode } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import type { PlanReviewPhase } from "../../hooks/usePlanReview";

interface PlanReviewCardProps {
  /** The merged plan text to show above the selection. */
  planText: string;
  /** Current review phase. */
  phase: PlanReviewPhase;
  /** Seconds remaining on the auto-accept countdown. */
  countdown: number;
  /** Accept the plan immediately. */
  onAccept: () => void;
  /** Enter editing mode (stops countdown). */
  onEdit: () => void;
  /** Submit feedback for the plan. */
  onSubmitFeedback: (feedback: string) => void;
}

/**
 * Replaces the composer at the bottom during plan review.
 * Shows the merged plan in a collapsible section, then a selection prompt.
 * Option 1 accepts; option 2 is an inline text input for feedback.
 */
export function PlanReviewCard({
  planText,
  phase,
  countdown,
  onAccept,
  onEdit,
  onSubmitFeedback,
}: PlanReviewCardProps): ReactNode {
  const [selected, setSelected] = useState<1 | 2>(1);
  const [feedback, setFeedback] = useState("");
  const [planOpen, setPlanOpen] = useState(true);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const isEditing = phase === "editing";
  const isReviewing = phase === "reviewing";

  // Focus the input when entering editing mode.
  useEffect(() => {
    if (isEditing) {
      inputRef.current?.focus();
    }
  }, [isEditing]);

  const submit = useCallback(() => {
    if (selected === 1) {
      onAccept();
    } else {
      onEdit();
    }
  }, [selected, onAccept, onEdit]);

  // Keyboard navigation — only active during reviewing (not editing).
  useEffect(() => {
    if (!isReviewing) return;

    function handleKey(event: KeyboardEvent): void {
      if (event.key === "ArrowUp" || event.key === "ArrowDown") {
        event.preventDefault();
        setSelected((prev) => (prev === 1 ? 2 : 1));
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        submit();
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        onEdit();
      }
    }

    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [isReviewing, submit, onEdit]);

  function handleFeedbackKeyDown(event: React.KeyboardEvent<HTMLInputElement>): void {
    if (event.key !== "Enter") return;
    event.preventDefault();
    const trimmed = feedback.trim();
    if (!trimmed) return;
    onSubmitFeedback(trimmed);
    setFeedback("");
  }

  const optionBase =
    "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors select-none";
  const optionActive = "bg-elevated text-fg";
  const optionInactive = "text-fg-muted";

  return (
    <div className="bg-surface px-5 pb-2 pt-1">
      {/* Merged plan panel — separate card above the selection */}
      {planText && (
        <div className="mb-2 rounded-[20px] border border-edge bg-panel shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
          <button
            type="button"
            onClick={() => setPlanOpen((prev) => !prev)}
            className="flex w-full items-center justify-between px-4 py-2.5"
          >
            <span className="text-sm font-semibold text-fg">Plan</span>
            <ChevronDown
              size={14}
              className={`text-fg-muted transition-transform ${planOpen ? "rotate-180" : ""}`}
            />
          </button>
          {planOpen && (
            <div className="max-h-[40vh] overflow-y-auto border-t border-edge px-4 py-3 pipeline-scroll">
              <p className="text-xs leading-5 text-fg-muted whitespace-pre-wrap">{planText}</p>
            </div>
          )}
        </div>
      )}

      {/* Selection card */}
      <div className="rounded-[20px] border border-edge bg-panel shadow-[0_0_0_1px_rgba(49,49,52,0.24)]">
        {/* Title */}
        <div className="px-4 pt-3 pb-0.5">
          <p className="text-sm font-semibold text-fg">Implement this plan?</p>
        </div>

        {/* Options */}
        <div className="flex flex-col gap-0.5 px-2 pb-1.5">
          {/* Option 1 — Accept */}
          <div
            role="button"
            tabIndex={0}
            onClick={onAccept}
            onMouseEnter={() => { if (isReviewing) setSelected(1); }}
            className={`${optionBase} cursor-pointer ${selected === 1 && isReviewing ? optionActive : optionInactive}`}
          >
            <span className="font-mono text-xs text-fg-faint">1.</span>
            <span className={selected === 1 && isReviewing ? "font-semibold" : ""}>
              Yes, implement this plan
              {isReviewing && countdown > 0 && (
                <span className="ml-1 text-fg-muted font-normal">({String(countdown)}s)</span>
              )}
            </span>
            {selected === 1 && isReviewing && (
              <span className="ml-auto flex items-center gap-0.5 text-fg-faint">
                <kbd className="inline-flex h-5 min-w-[1.25rem] items-center justify-center rounded border border-edge bg-surface px-1 text-[10px] font-mono">&uarr;</kbd>
                <kbd className="inline-flex h-5 min-w-[1.25rem] items-center justify-center rounded border border-edge bg-surface px-1 text-[10px] font-mono">&darr;</kbd>
              </span>
            )}
          </div>

          {/* Option 2 — inline text input for feedback */}
          <div
            role="button"
            tabIndex={0}
            onClick={() => { if (isReviewing) onEdit(); }}
            onMouseEnter={() => { if (isReviewing) setSelected(2); }}
            className={`${optionBase} ${isEditing ? optionActive : (selected === 2 && isReviewing ? optionActive : optionInactive)}`}
          >
            <span className="font-mono text-xs text-fg-faint">2.</span>
            {isEditing ? (
              <input
                ref={inputRef}
                type="text"
                value={feedback}
                onChange={(event) => setFeedback(event.target.value)}
                onKeyDown={handleFeedbackKeyDown}
                placeholder="Tell me what to do differently..."
                className="flex-1 bg-transparent text-sm text-fg placeholder:text-fg-faint focus:outline-none"
              />
            ) : (
              <span className={selected === 2 && isReviewing ? "font-semibold" : ""}>
                No, and tell me what to do differently
              </span>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 border-t border-edge px-3 py-2">
          <button
            type="button"
            onClick={() => {
              if (isEditing) {
                const trimmed = feedback.trim();
                if (!trimmed) return;
                onSubmitFeedback(trimmed);
                setFeedback("");
              } else {
                submit();
              }
            }}
            className="inline-flex items-center gap-1.5 rounded-lg bg-running-dot px-3 py-1.5 text-xs font-semibold text-send-text transition-colors hover:bg-send-bg-hover"
          >
            Submit
            <kbd className="inline-flex h-5 items-center rounded border border-white/20 bg-white/10 px-1.5 text-[10px] font-mono">&crarr;</kbd>
          </button>
        </div>
      </div>
    </div>
  );
}
