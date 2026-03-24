import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ApiHealth, ProviderInfo } from "../types";

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
  const [checking, setChecking] = useState<boolean>(false);

  useEffect(() => {
    const unHealth = listen<ApiHealth>("api_health_status", (event) => {
      setHealth(event.payload);
    });

    const unProvider = listen<ProviderInfo>("api_provider_info", (event) => {
      setProviders((prev) => {
        const filtered = prev.filter((p) => p.name !== event.payload.name);
        return [...filtered, event.payload];
      });
    });

    const unDone = listen<void>("api_providers_check_complete", () => {
      setChecking(false);
    });

    return () => {
      void unHealth.then((fn) => fn());
      void unProvider.then((fn) => fn());
      void unDone.then((fn) => fn());
    };
  }, []);

  const checkHealth = useCallback((): void => {
    setChecking(true);
    Promise.all([
      invoke("check_api_health"),
      invoke("get_api_providers"),
    ]).catch(() => {
      setChecking(false);
    });
  }, []);

  return { health, providers, checking, checkHealth };
}
