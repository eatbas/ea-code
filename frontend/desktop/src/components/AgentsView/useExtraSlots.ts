import { useEffect, useState } from "react";
import type { MutableRefObject } from "react";
import type { AppSettings, ExtraSlotConfig } from "../../types";

interface UseGroupedExtraSlotsResult {
  /** How many extra slots are currently visible in the UI. */
  openCount: number;
  /** How many extra slots have an agent configured. */
  activeCount: number;
  /** Open another empty slot (unlimited). */
  addSlot: () => void;
  /** Remove slot at index, shifting higher slots down. */
  removeSlot: (index: number) => void;
  /** Update the agent and model for a specific extra slot (writes both arrays). */
  updateSlot: (index: number, agent: string | null, model: string | null) => void;
}

/**
 * Manages grouped extra planner+reviewer slots in lockstep.
 * Both `extraPlanners` and `extraReviewers` arrays are kept identical.
 * No maximum limit — users can add as many as they want.
 */
export function useGroupedExtraSlots(
  settings: AppSettings,
  draftRef: MutableRefObject<AppSettings>,
  onUpdate: (patch: Partial<AppSettings>) => void,
): UseGroupedExtraSlotsResult {
  const arr = settings.extraPlanners as ExtraSlotConfig[];

  const [openCount, setOpenCount] = useState<number>(() => {
    return arr.filter((s) => s.agent != null).length;
  });

  // Re-sync when settings change externally (e.g. fresh start).
  useEffect(() => {
    const configured = arr.filter((s) => s.agent != null).length;
    setOpenCount((prev) => Math.max(prev, configured));
  }, [arr]);

  const activeCount = arr.filter((s) => s.agent != null).length;

  function addSlot(): void {
    setOpenCount((prev) => prev + 1);
  }

  function removeSlot(index: number): void {
    const planners = [...(draftRef.current.extraPlanners as ExtraSlotConfig[])];
    const reviewers = [...(draftRef.current.extraReviewers as ExtraSlotConfig[])];
    planners.splice(index, 1);
    reviewers.splice(index, 1);
    setOpenCount((prev) => Math.max(0, prev - 1));
    onUpdate({ extraPlanners: planners, extraReviewers: reviewers } as Partial<AppSettings>);
  }

  function updateSlot(index: number, agent: string | null, model: string | null): void {
    const planners = [...(draftRef.current.extraPlanners as ExtraSlotConfig[])];
    const reviewers = [...(draftRef.current.extraReviewers as ExtraSlotConfig[])];
    // Ensure arrays are long enough.
    while (planners.length <= index) planners.push({ agent: null, model: null });
    while (reviewers.length <= index) reviewers.push({ agent: null, model: null });
    const slot = { agent, model };
    planners[index] = slot;
    reviewers[index] = { ...slot };
    onUpdate({ extraPlanners: planners, extraReviewers: reviewers } as Partial<AppSettings>);
  }

  return {
    openCount,
    activeCount,
    addSlot,
    removeSlot,
    updateSlot,
  };
}
