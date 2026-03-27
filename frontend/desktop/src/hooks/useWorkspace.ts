import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectEntry, WorkspaceInfo } from "../types";
import { useToast } from "../components/shared/Toast";

interface UseWorkspaceReturn {
  workspace: WorkspaceInfo | null;
  error: string | null;
  openingWorkspace: boolean;
  openWorkspace: (path: string) => Promise<void>;
  selectFolder: () => Promise<void>;
  projects: ProjectEntry[];
  loadProjects: () => Promise<void>;
  deleteProject: (projectPath: string) => Promise<void>;
}

/** Hook to manage workspace folder selection via the native dialog. */
export function useWorkspace(): UseWorkspaceReturn {
  const toast = useToast();
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openingWorkspace, setOpeningWorkspace] = useState(false);
  const [projects, setProjects] = useState<ProjectEntry[]>([]);

  const loadProjects = useCallback(async (): Promise<void> => {
    try {
      const list = await invoke<ProjectEntry[]>("list_projects");
      setProjects(list);
    } catch {
      // Silent — project list is non-critical.
    }
  }, []);

  const deleteProject = useCallback(async (projectPath: string): Promise<void> => {
    try {
      await invoke("delete_project", { projectPath });
      await loadProjects();
      toast.success("Project removed.");
    } catch {
      toast.error("Failed to remove project.");
    }
  }, [loadProjects, toast]);

  const openWorkspace = useCallback(async (path: string): Promise<void> => {
    setOpeningWorkspace(true);
    try {
      const info = await invoke<WorkspaceInfo>("select_workspace", { path });
      setWorkspace(info);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      toast.error("Failed to open project.");
    } finally {
      setOpeningWorkspace(false);
    }
  }, [toast]);

  const selectFolder = useCallback(async (): Promise<void> => {
    try {
      const selected = await open({ directory: true, multiple: false });

      if (selected === null) {
        return;
      }

      await openWorkspace(selected);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      toast.error("Failed to select project folder.");
    }
  }, [openWorkspace, toast]);

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
  }, []);

  return { workspace, error, openingWorkspace, openWorkspace, selectFolder, projects, loadProjects, deleteProject };
}
