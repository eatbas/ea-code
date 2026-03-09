import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, AllCliVersions } from "../types";

interface UseCliVersionsReturn {
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  fetchVersions: (settings: AppSettings) => Promise<void>;
  updateCli: (cliName: string, settings: AppSettings) => Promise<void>;
}

/** Hook to fetch CLI version information and trigger updates. */
export function useCliVersions(): UseCliVersionsReturn {
  const [versions, setVersions] = useState<AllCliVersions | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [updating, setUpdating] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchVersions = useCallback(async (settings: AppSettings): Promise<void> => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<AllCliVersions>("get_cli_versions", { settings });
      setVersions(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      console.error("Failed to fetch CLI versions:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  const updateCli = useCallback(async (cliName: string, settings: AppSettings): Promise<void> => {
    setUpdating(cliName);
    setError(null);
    try {
      await invoke<string>("update_cli", { cliName });
      // Refresh version info after update
      const result = await invoke<AllCliVersions>("get_cli_versions", { settings });
      setVersions(result);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      console.error(`Failed to update ${cliName}:`, err);
    } finally {
      setUpdating(null);
    }
  }, []);

  return { versions, loading, updating, error, fetchVersions, updateCli };
}
