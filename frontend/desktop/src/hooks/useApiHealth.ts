import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ApiHealth, ProviderInfo } from "../types";
import { useTauriEventListeners } from "./useTauriEventListeners";

interface UseApiHealthReturn {
  health: ApiHealth | null;
  providers: ProviderInfo[];
  checking: boolean;
  checkHealth: () => void;
}

/** Hook to check hive-api health and provider availability (event-driven). */
export function useApiHealth(): UseApiHealthReturn {
  const [health, setHealth] = useState<ApiHealth | null>(null);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);

  const { checking, setChecking } = useTauriEventListeners({
    listeners: [
      {
        event: "api_health_status",
        handler: (payload: ApiHealth) => {
          setHealth(payload);
        },
      },
      {
        event: "api_provider_info",
        handler: (payload: ProviderInfo) => {
          setProviders((prev) => {
            const filtered = prev.filter((p) => p.name !== payload.name);
            return [...filtered, payload];
          });
        },
      },
    ],
    doneEvent: "api_providers_check_complete",
  });

  const checkHealth = useCallback((): void => {
    setChecking(true);
    Promise.all([
      invoke("check_api_health"),
      invoke("get_api_providers"),
    ]).catch(() => {
      setChecking(false);
    });
  }, [setChecking]);

  return { health, providers, checking, checkHealth };
}
