import { useState, useCallback, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { WorkspaceInfo } from "../types";

/** Minimum interval between automatic git status refreshes (milliseconds). */
const REFRESH_DEBOUNCE_MS = 2_000;

interface UseWorkspaceReturn {
  workspace: WorkspaceInfo | null;
  error: string | null;
  openWorkspace: (path: string) => Promise<void>;
  selectFolder: () => Promise<void>;
  refreshWorkspace: () => Promise<void>;
}

/** Hook to manage workspace folder selection and automatic git status refresh. */
export function useWorkspace(): UseWorkspaceReturn {
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const workspacePathRef = useRef<string | null>(null);
  const lastRefreshRef = useRef<number>(0);

  const openWorkspace = useCallback(async (path: string): Promise<void> => {
    try {
      const info = await invoke<WorkspaceInfo>("select_workspace", { path });
      setWorkspace(info);
      setError(null);
      workspacePathRef.current = path;
      lastRefreshRef.current = Date.now();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const refreshWorkspace = useCallback(async (): Promise<void> => {
    const path = workspacePathRef.current;
    if (!path) return;

    try {
      const info = await invoke<WorkspaceInfo>("refresh_workspace", { path });
      setWorkspace(info);
      lastRefreshRef.current = Date.now();
    } catch (err) {
      console.warn("Failed to refresh workspace status:", err);
    }
  }, []);

  const selectFolder = useCallback(async (): Promise<void> => {
    try {
      const selected = await open({ directory: true, multiple: false });

      if (selected === null) {
        return;
      }

      const path = typeof selected === "string" ? selected : selected;
      await openWorkspace(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [openWorkspace]);

  // Refresh git status when the Tauri window regains focus
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setup = async (): Promise<void> => {
      unlisten = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
        if (!focused) return;
        if (!workspacePathRef.current) return;

        const elapsed = Date.now() - lastRefreshRef.current;
        if (elapsed < REFRESH_DEBOUNCE_MS) return;

        void refreshWorkspace();
      });
    };

    void setup();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [refreshWorkspace]);

  return { workspace, error, openWorkspace, selectFolder, refreshWorkspace };
}
