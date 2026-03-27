import { useState, useCallback } from "react";
import type { ApiCliVersionInfo } from "../types";
import { API_EVENTS } from "../constants/events";
import { getApiCliVersions, updateApiCli } from "../lib/desktopApi";
import { useEventList } from "./useEventResource";

interface UseApiCliVersionsReturn {
  versions: ApiCliVersionInfo[];
  loading: boolean;
  updating: string | null;
  fetchVersions: () => void;
  updateCli: (provider: string) => Promise<void>;
}

/** Hook for CLI version info from hive-api (event-driven). */
export function useApiCliVersions(): UseApiCliVersionsReturn {
  const [updating, setUpdating] = useState<string | null>(null);
  const {
    state: versions,
    loading,
    setLoading,
  } = useEventList<ApiCliVersionInfo, string>({
    itemEvent: API_EVENTS.CLI_VERSION_INFO,
    doneEvent: API_EVENTS.CLI_VERSIONS_COMPLETE,
    getKey: (version) => version.provider,
  });

  const fetchVersions = useCallback((): void => {
    setLoading(true);
    getApiCliVersions().catch(() => {
      setLoading(false);
    });
  }, [setLoading]);

  const updateCli = useCallback(async (provider: string): Promise<void> => {
    setUpdating(provider);
    try {
      await updateApiCli(provider);
    } finally {
      setUpdating(null);
    }
  }, []);

  return { versions, loading, updating, fetchVersions, updateCli };
}
