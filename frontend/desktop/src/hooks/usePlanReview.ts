import { useCallback, useEffect, useRef, useState } from "react";
import { useToast } from "../components/shared/Toast";

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

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message) {
    return error.message;
  }
  if (typeof error === "string" && error.trim().length > 0) {
    return error;
  }
  return fallback;
}

export function usePlanReview({
  awaitingReview,
  running,
  onAccept,
  onSubmitFeedback,
}: UsePlanReviewOptions): UsePlanReviewReturn {
  const toast = useToast();
  const [phase, setPhase] = useState<PlanReviewPhase>("inactive");
  const [countdown, setCountdown] = useState(COUNTDOWN_SECONDS);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const onAcceptRef = useRef(onAccept);
  onAcceptRef.current = onAccept;
  const onSubmitFeedbackRef = useRef(onSubmitFeedback);
  onSubmitFeedbackRef.current = onSubmitFeedback;
  const prevAwaitingRef = useRef(false);
  const awaitingReviewRef = useRef(awaitingReview);
  awaitingReviewRef.current = awaitingReview;
  const runningRef = useRef(running);
  runningRef.current = running;
  const acceptingRef = useRef(false);
  const submittingFeedbackRef = useRef(false);
  const acceptActionRef = useRef<() => void>(() => {});

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
          acceptActionRef.current();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);
  }, [clearTimer]);

  const restoreReviewState = useCallback(() => {
    if (awaitingReviewRef.current) {
      setPhase("reviewing");
      startCountdown();
      return;
    }

    clearTimer();
    setPhase("inactive");
    setCountdown(COUNTDOWN_SECONDS);
  }, [clearTimer, startCountdown]);

  // When awaitingReview transitions from false to true, start reviewing.
  // We track the previous value so that a phase change alone (e.g.
  // submitting_edit while awaitingReview is still true) does not
  // immediately re-enter the reviewing state.
  useEffect(() => {
    const wasAwaiting = prevAwaitingRef.current;
    prevAwaitingRef.current = awaitingReview;

    if (awaitingReview && !wasAwaiting && (phase === "inactive" || phase === "submitting_edit")) {
      setPhase("reviewing");
      startCountdown();
    }
  }, [awaitingReview, phase, startCountdown]);

  // If review mode disappears unexpectedly, clear the stale review UI.
  useEffect(() => {
    if (awaitingReview || running) {
      return;
    }

    if (phase === "reviewing" || phase === "editing" || phase === "submitting_edit") {
      clearTimer();
      setPhase("inactive");
      setCountdown(COUNTDOWN_SECONDS);
    }
  }, [awaitingReview, running, phase, clearTimer]);

  // When pipeline starts running during an edit, move to submitting_edit.
  // When pipeline starts running after acceptance (or if a stale
  // loadFromSaved re-triggers the reviewing phase while the coding stage
  // is already active), go back to inactive so the PlanReviewCard
  // disappears and the coding stages render.
  useEffect(() => {
    if (running && phase === "editing") {
      setPhase("submitting_edit");
    } else if (running && (phase === "accepted" || phase === "reviewing")) {
      clearTimer();
      setPhase("inactive");
    }
  }, [running, phase, clearTimer]);

  // When pipeline stops running and conversation goes back to awaiting_review
  // after a submitting_edit round, the awaitingReview effect above handles it.

  // Clean up interval on unmount.
  useEffect(() => clearTimer, [clearTimer]);

  const startEdit = useCallback(() => {
    clearTimer();
    setPhase("editing");
  }, [clearTimer]);

  const accept = useCallback(() => {
    if (acceptingRef.current || !awaitingReviewRef.current || runningRef.current) {
      return;
    }

    acceptingRef.current = true;
    clearTimer();
    setPhase("accepted");
    void onAcceptRef.current()
      .catch((error: unknown) => {
        restoreReviewState();
        toast.error(getErrorMessage(error, "Failed to accept plan."));
      })
      .finally(() => {
        acceptingRef.current = false;
      });
  }, [clearTimer, restoreReviewState, toast]);
  acceptActionRef.current = accept;

  const submitFeedback = useCallback((feedback: string) => {
    const trimmed = feedback.trim();
    if (!trimmed || submittingFeedbackRef.current || !awaitingReviewRef.current) {
      return;
    }

    submittingFeedbackRef.current = true;
    clearTimer();
    setPhase("submitting_edit");
    void onSubmitFeedbackRef.current(trimmed)
      .catch((error: unknown) => {
        restoreReviewState();
        toast.error(getErrorMessage(error, "Failed to update plan."));
      })
      .finally(() => {
        submittingFeedbackRef.current = false;
      });
  }, [clearTimer, restoreReviewState, toast]);

  const reset = useCallback(() => {
    clearTimer();
    acceptingRef.current = false;
    submittingFeedbackRef.current = false;
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
