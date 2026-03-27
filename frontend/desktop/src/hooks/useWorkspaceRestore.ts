import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectEntry, WorkspaceInfo } from "../types";

interface UseWorkspaceRestoreOptions {
  /** Callback to replace the project list in state. */
  setProjects: (projects: ProjectEntry[]) => void;
  /** Callback to set the active workspace. */
  setWorkspace: (workspace: WorkspaceInfo) => void;
  /** Callback to set the error message. */
  setError: (error: string | null) => void;
  /** Callback to toggle the loading overlay. */
  setOpeningWorkspace: (opening: boolean) => void;
}

/**
 * Restores the most recently used workspace on mount.
 *
 * Loads the project list and, if a project exists, selects the first (most
 * recent) one as the active workspace. All state mutations are delegated to
 * the provided callbacks so this hook owns no state of its own.
 */
export function useWorkspaceRestore(options: UseWorkspaceRestoreOptions): void {
  const { setProjects, setWorkspace, setError, setOpeningWorkspace } = options;

  useEffect(() => {
    let disposed = false;

    async function restoreLastWorkspace(): Promise<void> {
      try {
        const list = await invoke<ProjectEntry[]>("list_projects");
        if (!disposed) {
          setProjects(list);
        }
        const lastProjectPath = list[0]?.path;
        if (!lastProjectPath || disposed) {
          return;
        }

        setOpeningWorkspace(true);
        const info = await invoke<WorkspaceInfo>("select_workspace", { path: lastProjectPath });
        if (disposed) {
          return;
        }

        setWorkspace(info);
        setError(null);
      } catch {
        // Ignore startup restore errors.
      } finally {
        if (!disposed) {
          setOpeningWorkspace(false);
        }
      }
    }

    void restoreLastWorkspace();
    return () => {
      disposed = true;
    };
  }, [setProjects, setWorkspace, setError, setOpeningWorkspace]);
}
