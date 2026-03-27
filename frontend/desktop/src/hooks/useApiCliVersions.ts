import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ApiCliVersionInfo } from "../types";
import { useTauriEventListeners } from "./useTauriEventListeners";

interface UseApiCliVersionsReturn {
  versions: ApiCliVersionInfo[];
  loading: boolean;
  updating: string | null;
  fetchVersions: () => void;
  updateCli: (provider: string) => Promise<void>;
}

/** Hook for CLI version info from hive-api (event-driven). */
export function useApiCliVersions(): UseApiCliVersionsReturn {
  const [versions, setVersions] = useState<ApiCliVersionInfo[]>([]);
  const [updating, setUpdating] = useState<string | null>(null);

  const { checking: loading, setChecking: setLoading } = useTauriEventListeners({
    listeners: [
      {
        event: "api_cli_version_info",
        handler: (payload: ApiCliVersionInfo) => {
          setVersions((prev) => {
            const filtered = prev.filter((v) => v.provider !== payload.provider);
            return [...filtered, payload];
          });
        },
      },
    ],
    doneEvent: "api_versions_check_complete",
  });

  const fetchVersions = useCallback((): void => {
    setLoading(true);
    invoke("get_api_cli_versions").catch(() => {
      setLoading(false);
    });
  }, [setLoading]);

  const updateCli = useCallback(async (provider: string): Promise<void> => {
    setUpdating(provider);
    try {
      await invoke("update_api_cli", { provider });
    } finally {
      setUpdating(null);
    }
  }, []);

  return { versions, loading, updating, fetchVersions, updateCli };
}
