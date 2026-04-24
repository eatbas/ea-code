import { useState, useCallback, useEffect, useRef } from "react";
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

/** Hook for CLI version info from Symphony (event-driven).
 *
 *  `loading` stays true from mount until the first fetch completes, so the
 *  UI renders a skeleton placeholder rather than "N/A" during the initial
 *  discovery window — "never fetched" stays distinguishable from "fetched
 *  with empty results". */
export function useApiCliVersions(): UseApiCliVersionsReturn {
  const [updating, setUpdating] = useState<string | null>(null);
  const [hasFetched, setHasFetched] = useState(false);
  const {
    state: versions,
    loading: rawLoading,
    setLoading,
  } = useEventList<ApiCliVersionInfo, string>({
    itemEvent: API_EVENTS.CLI_VERSION_INFO,
    doneEvent: API_EVENTS.CLI_VERSIONS_COMPLETE,
    getKey: (version) => version.provider,
  });

  // Mark as fetched the first time loading transitions true → false.
  const wasLoadingRef = useRef(false);
  useEffect(() => {
    if (wasLoadingRef.current && !rawLoading) {
      setHasFetched(true);
    }
    wasLoadingRef.current = rawLoading;
  }, [rawLoading]);

  const fetchVersions = useCallback((): void => {
    setLoading(true);
    getApiCliVersions().catch(() => {
      setLoading(false);
      setHasFetched(true);
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

  return {
    versions,
    loading: rawLoading || !hasFetched,
    updating,
    fetchVersions,
    updateCli,
  };
}
