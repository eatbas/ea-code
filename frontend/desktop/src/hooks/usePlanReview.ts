import { useCallback, useEffect, useRef, useState } from "react";

export type PlanReviewPhase =
  | "inactive"
  | "reviewing"
  | "editing"
  | "submitting_edit"
  | "accepted";

const COUNTDOWN_SECONDS = 35;

export interface UsePlanReviewReturn {
  phase: PlanReviewPhase;
  /** Seconds remaining on the auto-accept countdown (0..35). */
  countdown: number;
  /** Transition to editing mode (stops countdown). */
  startEdit: () => void;
  /** Accept the plan immediately. */
  accept: () => void;
  /** Submit edit feedback (transitions to submitting_edit). */
  submitFeedback: (feedback: string) => void;
  /** Reset to inactive. */
  reset: () => void;
}

interface UsePlanReviewOptions {
  /** Whether the pipeline is in awaiting_review status. */
  awaitingReview: boolean;
  /** Whether the pipeline is currently running (e.g. during an edit round). */
  running: boolean;
  /** Callback: accept the plan. */
  onAccept: () => Promise<void>;
  /** Callback: send edit feedback to the merge agent. */
  onSubmitFeedback: (feedback: string) => Promise<void>;
}

export function usePlanReview({
  awaitingReview,
  running,
  onAccept,
  onSubmitFeedback,
}: UsePlanReviewOptions): UsePlanReviewReturn {
  const [phase, setPhase] = useState<PlanReviewPhase>("inactive");
  const [countdown, setCountdown] = useState(COUNTDOWN_SECONDS);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onAcceptRef = useRef(onAccept);
  onAcceptRef.current = onAccept;
  const prevAwaitingRef = useRef(false);

  const clearTimer = useCallback(() => {
    if (intervalRef.current !== null) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }
  }, []);

  const startCountdown = useCallback(() => {
    clearTimer();
    setCountdown(COUNTDOWN_SECONDS);
    intervalRef.current = setInterval(() => {
      setCountdown((prev) => {
        if (prev <= 1) {
          clearTimer();
          // Auto-accept when countdown reaches zero.
          void onAcceptRef.current();
          setPhase("accepted");
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
  }, [clearTimer]);

  // When awaitingReview transitions from false→true → start reviewing.
  // We track the previous value so that a phase change alone (e.g.
  // submitting_edit while awaitingReview is still true) doesn't
  // immediately re-enter the reviewing state.
  useEffect(() => {
    const wasAwaiting = prevAwaitingRef.current;
    prevAwaitingRef.current = awaitingReview;

    if (awaitingReview && !wasAwaiting && (phase === "inactive" || phase === "submitting_edit")) {
      setPhase("reviewing");
      startCountdown();
    }
  }, [awaitingReview, phase, startCountdown]);

  // When pipeline starts running during an edit → move to submitting_edit.
  // When pipeline starts running after acceptance → go back to inactive
  // so the PlanReviewCard disappears and the coding stages render.
  useEffect(() => {
    if (running && phase === "editing") {
      setPhase("submitting_edit");
    } else if (running && phase === "accepted") {
      setPhase("inactive");
    }
  }, [running, phase]);

  // When pipeline stops running and conversation goes back to awaiting_review
  // after a submitting_edit round → handled by the awaitingReview effect above.

  // Clean up interval on unmount.
  useEffect(() => clearTimer, [clearTimer]);

  const startEdit = useCallback(() => {
    clearTimer();
    setPhase("editing");
  }, [clearTimer]);

  const accept = useCallback(() => {
    clearTimer();
    setPhase("accepted");
    void onAcceptRef.current();
  }, [clearTimer]);

  const submitFeedback = useCallback((feedback: string) => {
    setPhase("submitting_edit");
    void onSubmitFeedback(feedback);
  }, [onSubmitFeedback]);

  const reset = useCallback(() => {
    clearTimer();
    setPhase("inactive");
    setCountdown(COUNTDOWN_SECONDS);
  }, [clearTimer]);

  return {
    phase,
    countdown,
    startEdit,
    accept,
    submitFeedback,
    reset,
  };
}
