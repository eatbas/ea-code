import { useCallback } from "react";
import type { ApiHealth, ProviderInfo } from "../types";
import { API_EVENTS } from "../constants/events";
import { refreshApiProviders } from "../lib/desktopApi";
import { useEventList, useEventValue } from "./useEventResource";

interface UseApiHealthReturn {
  health: ApiHealth | null;
  providers: ProviderInfo[];
  checking: boolean;
  checkHealth: () => void;
}

/** Hook to check Symphony health and provider availability (event-driven). */
export function useApiHealth(): UseApiHealthReturn {
  const { state: health } = useEventValue<ApiHealth | null>({
    initialValue: null,
    itemEvent: API_EVENTS.HEALTH_STATUS,
  });
  const {
    state: providers,
    loading: checking,
    setLoading: setChecking,
  } = useEventList<ProviderInfo, string>({
    itemEvent: API_EVENTS.PROVIDER_INFO,
    doneEvent: API_EVENTS.PROVIDERS_COMPLETE,
    getKey: (provider) => provider.name,
  });

  const checkHealth = useCallback((): void => {
    setChecking(true);
    refreshApiProviders().catch(() => {
      setChecking(false);
    });
  }, [setChecking]);

  return { health, providers, checking, checkHealth };
}
