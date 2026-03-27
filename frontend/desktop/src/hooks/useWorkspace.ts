import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProjectEntry, WorkspaceInfo } from "../types";
import { useToast } from "../components/shared/Toast";
import { useProjectList } from "./useProjectList";
import { useWorkspaceRestore } from "./useWorkspaceRestore";

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

  const { projects, loadProjects, deleteProject, setProjects } = useProjectList();

  useWorkspaceRestore({
    setProjects,
    setWorkspace,
    setError,
    setOpeningWorkspace,
  });

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

  return { workspace, error, openingWorkspace, openWorkspace, selectFolder, projects, loadProjects, deleteProject };
}
