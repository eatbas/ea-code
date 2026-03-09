import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, AllCliVersions } from "../types";
import { useToast } from "../components/shared/Toast";

interface CliActionResult {
  success: boolean;
  message?: string;
}

interface UseCliVersionsReturn {
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  fetchVersions: (settings: AppSettings) => Promise<CliActionResult>;
  updateCli: (cliName: string, settings: AppSettings) => Promise<CliActionResult>;
}

/** Hook to fetch CLI version information and trigger updates. */
export function useCliVersions(): UseCliVersionsReturn {
  const toast = useToast();
  const [versions, setVersions] = useState<AllCliVersions | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [updating, setUpdating] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const fetchVersions = useCallback(async (settings: AppSettings): Promise<CliActionResult> => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<AllCliVersions>("get_cli_versions", { settings });
      setVersions(result);
      return { success: true };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toast.error("Failed to fetch CLI versions.");
      return { success: false, message };
    } finally {
      setLoading(false);
    }
  }, [toast]);

  const updateCli = useCallback(async (cliName: string, settings: AppSettings): Promise<CliActionResult> => {
    setUpdating(cliName);
    setError(null);
    try {
      const updateResult = await invoke<string>("update_cli", { cliName });
      // Refresh version info after update
      const result = await invoke<AllCliVersions>("get_cli_versions", { settings });
      setVersions(result);
      return { success: true, message: updateResult };
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toast.error(`Failed to update ${cliName}.`);
      return { success: false, message };
    } finally {
      setUpdating(null);
    }
  }, [toast]);

  return { versions, loading, updating, error, fetchVersions, updateCli };
}
