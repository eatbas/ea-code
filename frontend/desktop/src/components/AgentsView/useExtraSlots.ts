import { useEffect, useState } from "react";
import type { MutableRefObject } from "react";
import type { AgentBackend, AppSettings, ExtraSlotConfig } from "../../types";

interface UseExtraSlotsResult {
  /** How many extra slots are currently visible in the UI. */
  openCount: number;
  /** Maximum extra slots allowed (maxPlanners/maxReviewers - 1). */
  maxExtra: number;
  /** How many extra slots have an agent configured. */
  activeCount: number;
  /** Open another empty slot. */
  addSlot: () => void;
  /** Remove slot at index, shifting higher slots down. */
  removeSlot: (index: number) => void;
  /** Update the agent and model for a specific extra slot. */
  updateSlot: (index: number, agent: AgentBackend | null, model: string | null) => void;
}

/**
 * Manages dynamic extra planner or reviewer slot state.
 * Reads from `settings[arrayKey]` and `settings[maxKey]`.
 */
export function useExtraSlots(
  settings: AppSettings,
  draftRef: MutableRefObject<AppSettings>,
  onUpdate: (patch: Partial<AppSettings>) => void,
  arrayKey: "extraPlanners" | "extraReviewers",
  maxKey: "maxPlanners" | "maxReviewers",
): UseExtraSlotsResult {
  const maxTotal = settings[maxKey] as number;
  const maxExtra = Math.max(0, maxTotal - 1);
  const arr = settings[arrayKey] as ExtraSlotConfig[];

  const [openCount, setOpenCount] = useState<number>(() => {
    // Initialise to the number of configured slots.
    return Math.min(arr.filter((s) => s.agent != null).length, maxExtra);
  });

  // Re-sync when settings change externally (e.g. fresh start).
  useEffect(() => {
    const configured = arr.filter((s) => s.agent != null).length;
    setOpenCount((prev) => Math.max(prev, Math.min(configured, maxExtra)));
  }, [arr, maxExtra]);

  // Cap openCount to maxExtra if it decreases.
  useEffect(() => {
    setOpenCount((prev) => Math.min(prev, maxExtra));
  }, [maxExtra]);

  const activeCount = arr.filter((s) => s.agent != null).length;

  function addSlot(): void {
    if (openCount < maxExtra) {
      setOpenCount((prev) => prev + 1);
    }
  }

  function removeSlot(index: number): void {
    const current = [...(draftRef.current[arrayKey] as ExtraSlotConfig[])];
    // Remove the slot at index, shifting remaining down.
    current.splice(index, 1);
    setOpenCount((prev) => Math.max(0, prev - 1));
    onUpdate({ [arrayKey]: current } as Partial<AppSettings>);
  }

  function updateSlot(index: number, agent: AgentBackend | null, model: string | null): void {
    const current = [...(draftRef.current[arrayKey] as ExtraSlotConfig[])];
    // Ensure array is long enough.
    while (current.length <= index) {
      current.push({ agent: null, model: null });
    }
    current[index] = { agent, model };
    onUpdate({ [arrayKey]: current } as Partial<AppSettings>);
  }

  return {
    openCount,
    maxExtra,
    activeCount,
    addSlot,
    removeSlot,
    updateSlot,
  };
}
