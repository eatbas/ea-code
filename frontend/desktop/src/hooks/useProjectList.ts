import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectEntry } from "../types";
import { useToast } from "../components/shared/Toast";

interface UseProjectListReturn {
  projects: ProjectEntry[];
  loadProjects: () => Promise<void>;
  deleteProject: (projectPath: string) => Promise<void>;
  /** Directly replace the project list (used by startup restore). */
  setProjects: (projects: ProjectEntry[]) => void;
}

/** Hook for project list CRUD operations backed by the Tauri storage layer. */
export function useProjectList(): UseProjectListReturn {
  const toast = useToast();
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

  return { projects, loadProjects, deleteProject, setProjects };
}
