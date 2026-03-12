import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppSettings, AllCliVersions, CliVersionInfo } from "../types";
import { useToast } from "../components/shared/Toast";

interface UseCliVersionsReturn {
  versions: AllCliVersions | null;
  loading: boolean;
  updating: string | null;
  error: string | null;
  fetchVersions: (settings: AppSettings) => void;
  updateCli: (cliName: string, settings: AppSettings) => Promise<void>;
}

/** CLI names that map to `AllCliVersions` fields (excluding gitBash). */
const MAIN_CLI_NAMES = new Set(["claude", "codex", "gemini", "kimi", "opencode"]);

/** Hook to fetch CLI version information and trigger updates (event-driven). */
export function useCliVersions(): UseCliVersionsReturn {
  const toast = useToast();
  const [versions, setVersions] = useState<AllCliVersions | null>(null);
  const [loading, setLoading] = useState<boolean>(false);
  const [updating, setUpdating] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Per-CLI version events stream in as each check completes.
  useEffect(() => {
    const unRow = listen<CliVersionInfo>("cli_version_info", (event) => {
      const info = event.payload;
      setVersions((prev) => {
        const base = prev ?? {
          claude: placeholderInfo("Claude CLI", "claude"),
          codex: placeholderInfo("Codex CLI", "codex"),
          gemini: placeholderInfo("Gemini CLI", "gemini"),
          kimi: placeholderInfo("Kimi CLI", "kimi"),
          opencode: placeholderInfo("OpenCode CLI", "opencode"),
        };
        if (info.cliName === "gitBash") {
          return { ...base, gitBash: info };
        }
        if (MAIN_CLI_NAMES.has(info.cliName)) {
          return { ...base, [info.cliName]: info };
        }
        return base;
      });
    });
    const unDone = listen<void>("cli_versions_check_complete", () => {
      setLoading(false);
    });
    return () => {
      void unRow.then((fn) => fn());
      void unDone.then((fn) => fn());
    };
  }, []);

  // Fire-and-forget: starts the check and returns immediately.
  const fetchVersions = useCallback((settings: AppSettings): void => {
    setLoading(true);
    setError(null);
    invoke("get_cli_versions", { settings }).catch((err: unknown) => {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setLoading(false);
      toast.error("Failed to fetch CLI versions.");
    });
  }, [toast]);

  // Update is still awaitable — it's a single-CLI action, not a batch.
  const updateCli = useCallback(async (cliName: string, settings: AppSettings): Promise<void> => {
    setUpdating(cliName);
    setError(null);
    try {
      await invoke<string>("update_cli", { cliName });
      // Re-check all versions after update (fire-and-forget).
      fetchVersions(settings);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toast.error(`Failed to update ${cliName}.`);
    } finally {
      setUpdating(null);
    }
  }, [fetchVersions, toast]);

  return { versions, loading, updating, error, fetchVersions, updateCli };
}

function placeholderInfo(name: string, cliName: string): CliVersionInfo {
  return { name, cliName, upToDate: false, updateCommand: "", available: false };
}
