import { useCallback } from "react";
import type { AppSettings, CliHealth, CliStatus } from "../types";
import { CLI_EVENTS } from "../constants/events";
import { useToast } from "../components/shared/Toast";
import { checkCliHealth as requestCliHealth, invalidateCliCache } from "../lib/desktopApi";
import { useEventRecord } from "./useEventResource";

interface CliHealthEvent {
  cliName: keyof CliHealth;
  status: CliStatus;
}

const DEFAULT_STATUS: CliStatus = { available: false, path: "" };

const DEFAULT_HEALTH: CliHealth = {
  claude: { ...DEFAULT_STATUS },
  codex: { ...DEFAULT_STATUS },
  gemini: { ...DEFAULT_STATUS },
  kimi: { ...DEFAULT_STATUS },
  opencode: { ...DEFAULT_STATUS },
};

interface UseCliHealthReturn {
  health: CliHealth;
  checking: boolean;
  checkHealth: (settings: AppSettings) => void;
}

/** Hook to check availability of CLI agent backends (event-driven, non-blocking). */
export function useCliHealth(): UseCliHealthReturn {
  const toast = useToast();
  const {
    state: health,
    loading: checking,
    setLoading: setChecking,
  } = useEventRecord<CliStatus, CliHealthEvent, keyof CliHealth, CliHealth>({
    initialState: DEFAULT_HEALTH,
    itemEvent: CLI_EVENTS.HEALTH_STATUS,
    doneEvent: CLI_EVENTS.HEALTH_COMPLETE,
    getKey: (payload) => payload.cliName,
    getValue: (payload) => payload.status,
  });

  const checkHealth = useCallback((settings: AppSettings): void => {
    setChecking(true);
    void invalidateCliCache().catch(() => {
      // Best-effort cache invalidation.
    });
    requestCliHealth(settings).catch(() => {
      setChecking(false);
      toast.error("CLI health check failed.");
    });
  }, [setChecking, toast]);

  return { health, checking, checkHealth };
}
