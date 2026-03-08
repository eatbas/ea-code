import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { WorkspaceInfo } from "../types";

interface UseWorkspaceReturn {
  workspace: WorkspaceInfo | null;
  error: string | null;
  openWorkspace: (path: string) => Promise<void>;
  selectFolder: () => Promise<void>;
}

/** Hook to manage workspace folder selection via the native dialog. */
export function useWorkspace(): UseWorkspaceReturn {
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const openWorkspace = useCallback(async (path: string): Promise<void> => {
    try {
      const info = await invoke<WorkspaceInfo>("select_workspace", { path });
      setWorkspace(info);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  const selectFolder = useCallback(async (): Promise<void> => {
    try {
      const selected = await open({ directory: true, multiple: false });

      if (selected === null) {
        // User cancelled the dialog
        return;
      }

      const path = typeof selected === "string" ? selected : selected;
      await openWorkspace(path);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [openWorkspace]);

  return { workspace, error, openWorkspace, selectFolder };
}
