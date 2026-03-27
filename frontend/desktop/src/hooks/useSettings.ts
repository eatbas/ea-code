import { useState, useEffect, useCallback } from "react";
import type { AppSettings } from "../types";
import { useToast } from "../components/shared/Toast";
import { getSettings, saveSettings as persistSettings } from "../lib/desktopApi";

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
        const result = await getSettings();
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

    void loadSettings();

    return () => {
      cancelled = true;
    };
  }, [toast]);

  const saveSettings = useCallback(async (updated: AppSettings, options?: SaveSettingsOptions): Promise<void> => {
    try {
      await persistSettings(updated);
      setSettings(updated);
      setError(null);
      if (options?.notifySuccess) {
        toast.success("Settings saved.");
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      toast.error("Failed to save settings.");
    }
  }, [toast]);

  return { settings, loading, error, saveSettings };
}
