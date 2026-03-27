import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, CliHealth, CliStatus } from "../types";
import { useToast } from "../components/shared/Toast";
import { useTauriEventListeners } from "./useTauriEventListeners";

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

  const { checking, setChecking } = useTauriEventListeners({
    listeners: [
      {
        event: "cli_health_status",
        handler: (payload: CliHealthEvent) => {
          const { cliName, status } = payload;
          setHealth((prev) => ({
            ...(prev ?? DEFAULT_HEALTH),
            [cliName]: status,
          }));
        },
      },
    ],
    doneEvent: "cli_health_check_complete",
  });

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
  }, [toast, setChecking]);

  return { health, checking, checkHealth };
}
