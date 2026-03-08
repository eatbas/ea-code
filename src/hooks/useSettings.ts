import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";

interface UseSettingsReturn {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;
  saveSettings: (updated: AppSettings) => Promise<void>;
  clearProjectSettings: () => Promise<void>;
}

/** Hook to load and persist application settings via Tauri commands. */
export function useSettings(projectPath?: string): UseSettingsReturn {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadSettings(): Promise<void> {
      try {
        const result = projectPath
          ? await invoke<AppSettings>("get_project_settings", { projectPath })
          : await invoke<AppSettings>("get_settings");
        if (!cancelled) {
          setSettings(result);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    loadSettings();

    return () => {
      cancelled = true;
    };
  }, [projectPath]);

  const saveSettings = useCallback(async (updated: AppSettings): Promise<void> => {
    try {
      if (projectPath) {
        await invoke("save_project_settings", { projectPath, newSettings: updated });
      } else {
        await invoke("save_settings", { newSettings: updated });
      }
      setSettings(updated);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [projectPath]);

  const clearProjectSettings = useCallback(async (): Promise<void> => {
    if (!projectPath) {
      return;
    }
    try {
      await invoke("clear_project_settings", { projectPath });
      const refreshed = await invoke<AppSettings>("get_project_settings", { projectPath });
      setSettings(refreshed);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [projectPath]);

  return { settings, loading, error, saveSettings, clearProjectSettings };
}
