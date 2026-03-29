import { useEventValue } from "./useEventResource";
import { SIDECAR_EVENTS } from "../constants/events";

interface SidecarReadyPayload {
  ready: boolean;
  error?: string;
}

interface UseSidecarReadyReturn {
  /** `null` while the sidecar is still starting up. */
  sidecarReady: boolean | null;
  sidecarError: string | null;
}

/**
 * Listens for the one-shot `sidecar_ready` event emitted by the Rust backend
 * once the hive-api process is healthy (or has failed to start).
 */
export function useSidecarReady(): UseSidecarReadyReturn {
  const { state } = useEventValue<SidecarReadyPayload | null>({
    initialValue: null,
    itemEvent: SIDECAR_EVENTS.READY,
  });

  return {
    sidecarReady: state?.ready ?? null,
    sidecarError: state?.error ?? null,
  };
}
