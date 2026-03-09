import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types";
import { useToast } from "../components/shared/Toast";

interface SaveSettingsOptions {
  notifySuccess?: boolean;
}

interface UseSettingsReturn {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;
  saveSettings: (updated: AppSettings, options?: SaveSettingsOptions) => Promise<void>;
}

/** Hook to load and persist application settings via Tauri commands. */
export function useSettings(): UseSettingsReturn {
  const toast = useToast();
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadSettings(): Promise<void> {
      try {
        const result = await invoke<AppSettings>("get_settings");
        if (!cancelled) {
          setSettings(result);
          setError(null);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
          toast.error("Failed to load settings.");
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
  }, [toast]);

  const saveSettings = useCallback(async (updated: AppSettings, options?: SaveSettingsOptions): Promise<void> => {
    try {
      await invoke("save_settings", { newSettings: updated });
      setSettings(updated);
      setError(null);
      if (options?.notifySuccess) {
        toast.success("Settings saved.");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      toast.error("Failed to save settings.");
    }
  }, [toast]);

  return { settings, loading, error, saveSettings };
}
