import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, CliHealth } from "../types";

interface UseCliHealthReturn {
  health: CliHealth | null;
  checking: boolean;
  checkHealth: (settings: AppSettings) => Promise<void>;
}

/** Hook to check availability of CLI agent backends. */
export function useCliHealth(): UseCliHealthReturn {
  const [health, setHealth] = useState<CliHealth | null>(null);
  const [checking, setChecking] = useState<boolean>(false);

  const checkHealth = useCallback(async (settings: AppSettings): Promise<void> => {
    setChecking(true);
    try {
      const result = await invoke<CliHealth>("check_cli_health", { settings });
      setHealth(result);
    } catch (err) {
      console.error("CLI health check failed:", err);
    } finally {
      setChecking(false);
    }
  }, []);

  return { health, checking, checkHealth };
}
