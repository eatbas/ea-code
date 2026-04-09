import {
  createElement,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import type { ReactNode } from "react";
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

const SettingsContext = createContext<UseSettingsReturn | null>(null);

interface SettingsProviderProps {
  children: ReactNode;
}

/** Shared settings provider backed by the Tauri settings commands. */
export function SettingsProvider({ children }: SettingsProviderProps): ReactNode {
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

  const value = useMemo<UseSettingsReturn>(() => ({
    settings,
    loading,
    error,
    saveSettings,
  }), [error, loading, saveSettings, settings]);

  return createElement(SettingsContext.Provider, { value }, children);
}

/** Hook to access shared application settings loaded by `SettingsProvider`. */
export function useSettings(): UseSettingsReturn {
  const context = useContext(SettingsContext);

  if (!context) {
    throw new Error("useSettings must be used within SettingsProvider.");
  }

  return context;
}
