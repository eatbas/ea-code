import { useEffect, useState } from "react";
import type { MutableRefObject } from "react";
import type { AppSettings } from "../../types";

type ParallelSlot = "slot2" | "slot3";

interface ParallelSlotKeys {
  slot2AgentKey: keyof AppSettings;
  slot2ModelKey: keyof AppSettings;
  slot3AgentKey: keyof AppSettings;
  slot3ModelKey: keyof AppSettings;
}

interface ParallelSlotState {
  slot2: boolean;
  slot3: boolean;
}

interface UseParallelSlotGroupResult {
  extraSlots: ParallelSlotState;
  activeCount: number;
  openCount: number;
  addSlot: () => void;
  removeSlot: (slot: ParallelSlot) => void;
}

function countConfiguredSlots(settings: AppSettings, keys: ParallelSlotKeys): number {
  let count = 0;
  if (settings[keys.slot2AgentKey] != null) {
    count += 1;
  }
  if (settings[keys.slot3AgentKey] != null) {
    count += 1;
  }
  return count;
}

function createInitialState(settings: AppSettings, keys: ParallelSlotKeys): ParallelSlotState {
  return {
    slot2: settings[keys.slot2AgentKey] !== null,
    slot3: settings[keys.slot3AgentKey] !== null,
  };
}

export function useParallelSlotGroup(
  settings: AppSettings,
  draftRef: MutableRefObject<AppSettings>,
  onUpdate: (patch: Partial<AppSettings>) => void,
  keys: ParallelSlotKeys,
): UseParallelSlotGroupResult {
  const [extraSlots, setExtraSlots] = useState<ParallelSlotState>(() => createInitialState(settings, keys));

  useEffect(() => {
    setExtraSlots((prev) => ({
      slot2: prev.slot2 || settings[keys.slot2AgentKey] !== null,
      slot3: prev.slot3 || settings[keys.slot3AgentKey] !== null,
    }));
  }, [settings, keys.slot2AgentKey, keys.slot3AgentKey]);

  const activeCount = countConfiguredSlots(settings, keys);
  const openCount = (extraSlots.slot2 ? 1 : 0) + (extraSlots.slot3 ? 1 : 0);

  function addSlot(): void {
    if (!extraSlots.slot2) {
      setExtraSlots((prev) => ({ ...prev, slot2: true }));
      return;
    }

    if (!extraSlots.slot3) {
      setExtraSlots((prev) => ({ ...prev, slot3: true }));
    }
  }

  function removeSlot(slot: ParallelSlot): void {
    if (slot === "slot2" && extraSlots.slot3) {
      setExtraSlots((prev) => ({ ...prev, slot3: false }));
      onUpdate({
        [keys.slot2AgentKey]: draftRef.current[keys.slot3AgentKey],
        [keys.slot2ModelKey]: draftRef.current[keys.slot3ModelKey],
        [keys.slot3AgentKey]: null,
        [keys.slot3ModelKey]: null,
      } as Partial<AppSettings>);
      return;
    }

    setExtraSlots((prev) => ({ ...prev, [slot]: false }));
    if (slot === "slot2") {
      onUpdate({
        [keys.slot2AgentKey]: null,
        [keys.slot2ModelKey]: null,
      } as Partial<AppSettings>);
      return;
    }

    onUpdate({
      [keys.slot3AgentKey]: null,
      [keys.slot3ModelKey]: null,
    } as Partial<AppSettings>);
  }

  return {
    extraSlots,
    activeCount,
    openCount,
    addSlot,
    removeSlot,
  };
}
