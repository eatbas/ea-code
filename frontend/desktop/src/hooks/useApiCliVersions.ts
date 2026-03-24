import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ApiCliVersionInfo } from "../types";

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
  const [loading, setLoading] = useState<boolean>(false);
  const [updating, setUpdating] = useState<string | null>(null);

  useEffect(() => {
    const unVersion = listen<ApiCliVersionInfo>("api_cli_version_info", (event) => {
      setVersions((prev) => {
        const filtered = prev.filter((v) => v.provider !== event.payload.provider);
        return [...filtered, event.payload];
      });
    });

    const unDone = listen<void>("api_versions_check_complete", () => {
      setLoading(false);
    });

    return () => {
      void unVersion.then((fn) => fn());
      void unDone.then((fn) => fn());
    };
  }, []);

  const fetchVersions = useCallback((): void => {
    setLoading(true);
    invoke("get_api_cli_versions").catch(() => {
      setLoading(false);
    });
  }, []);

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
