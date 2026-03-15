import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppSettings, CliHealth, CliStatus } from "../types";
import { useToast } from "../components/shared/Toast";

interface CliHealthEvent {
  cliName: string;
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
  health: CliHealth | null;
  checking: boolean;
  checkHealth: (settings: AppSettings) => void;
}

/** Hook to check availability of CLI agent backends (event-driven, non-blocking). */
export function useCliHealth(): UseCliHealthReturn {
  const toast = useToast();
  const [health, setHealth] = useState<CliHealth | null>(null);
  const [checking, setChecking] = useState<boolean>(false);

  // Per-CLI events stream in as each binary check completes.
  useEffect(() => {
    const unRow = listen<CliHealthEvent>("cli_health_status", (event) => {
      const { cliName, status } = event.payload;
      setHealth((prev) => ({
        ...(prev ?? DEFAULT_HEALTH),
        [cliName]: status,
      }));
    });
    const unDone = listen<void>("cli_health_check_complete", () => {
      setChecking(false);
    });
    return () => {
      void unRow.then((fn) => fn());
      void unDone.then((fn) => fn());
    };
  }, []);

  const checkHealth = useCallback((settings: AppSettings): void => {
    setChecking(true);
    invoke("invalidate_cli_cache")
      .catch(() => {
        /* best-effort — continue even if invalidation fails */
      })
      .then(() => invoke("check_cli_health", { settings }))
      .catch(() => {
        setChecking(false);
        toast.error("CLI health check failed.");
      });
  }, [toast]);

  return { health, checking, checkHealth };
}
