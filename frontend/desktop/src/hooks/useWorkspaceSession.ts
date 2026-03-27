import { useCallback, useEffect, useReducer } from "react";
import type { ProjectEntry, WorkspaceInfo } from "../types";
import { useToast } from "../components/shared/Toast";
import {
  deleteProject as deleteProjectBookmark,
  listProjects,
  selectProjectFolder,
  selectWorkspace,
} from "../lib/desktopApi";

export interface WorkspaceSessionState {
  workspace: WorkspaceInfo | null;
  projects: ProjectEntry[];
  openingWorkspace: boolean;
  error: string | null;
}

export type WorkspaceSessionAction =
  | { type: "set-projects"; projects: ProjectEntry[] }
  | { type: "open-workspace:start" }
  | { type: "open-workspace:success"; workspace: WorkspaceInfo }
  | { type: "open-workspace:error"; error: string }
  | { type: "open-workspace:end" };

export interface UseWorkspaceSessionReturn extends WorkspaceSessionState {
  openWorkspace: (path: string) => Promise<void>;
  selectFolder: () => Promise<void>;
  refreshProjects: () => Promise<void>;
  deleteProject: (projectPath: string) => Promise<void>;
}

export function createWorkspaceSessionInitialState(): WorkspaceSessionState {
  return {
    workspace: null,
    projects: [],
    openingWorkspace: false,
    error: null,
  };
}

export function workspaceSessionReducer(
  state: WorkspaceSessionState,
  action: WorkspaceSessionAction,
): WorkspaceSessionState {
  switch (action.type) {
    case "set-projects":
      return {
        ...state,
        projects: action.projects,
      };
    case "open-workspace:start":
      return {
        ...state,
        openingWorkspace: true,
      };
    case "open-workspace:success":
      return {
        ...state,
        workspace: action.workspace,
        error: null,
      };
    case "open-workspace:error":
      return {
        ...state,
        error: action.error,
      };
    case "open-workspace:end":
      return {
        ...state,
        openingWorkspace: false,
      };
    default:
      return state;
  }
}

/** Owns workspace selection, project bookmarks, and startup restore. */
export function useWorkspaceSession(): UseWorkspaceSessionReturn {
  const toast = useToast();
  const [state, dispatch] = useReducer(
    workspaceSessionReducer,
    undefined,
    createWorkspaceSessionInitialState,
  );

  const refreshProjects = useCallback(async (): Promise<void> => {
    try {
      dispatch({ type: "set-projects", projects: await listProjects() });
    } catch {
      // Project list is non-critical; leave the existing snapshot in place.
    }
  }, []);

  const openWorkspaceInternal = useCallback(async (
    path: string,
    options?: { notifyError?: boolean; refreshProjects?: boolean },
  ): Promise<void> => {
    dispatch({ type: "open-workspace:start" });
    try {
      const workspace = await selectWorkspace(path);
      dispatch({ type: "open-workspace:success", workspace });
      if (options?.refreshProjects ?? true) {
        await refreshProjects();
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      dispatch({ type: "open-workspace:error", error: message });
      if (options?.notifyError ?? true) {
        toast.error("Failed to open project.");
      }
    } finally {
      dispatch({ type: "open-workspace:end" });
    }
  }, [refreshProjects, toast]);

  const openWorkspace = useCallback(async (path: string): Promise<void> => {
    await openWorkspaceInternal(path, { notifyError: true, refreshProjects: true });
  }, [openWorkspaceInternal]);

  const selectFolder = useCallback(async (): Promise<void> => {
    try {
      const selected = await selectProjectFolder();
      if (!selected) {
        return;
      }
      await openWorkspace(selected);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      dispatch({ type: "open-workspace:error", error: message });
      toast.error("Failed to select project folder.");
    }
  }, [openWorkspace, toast]);

  const deleteProject = useCallback(async (projectPath: string): Promise<void> => {
    try {
      await deleteProjectBookmark(projectPath);
      await refreshProjects();
      toast.success("Project removed.");
    } catch {
      toast.error("Failed to remove project.");
    }
  }, [refreshProjects, toast]);

  useEffect(() => {
    let cancelled = false;

    async function restoreWorkspaceSession(): Promise<void> {
      try {
        const projects = await listProjects();
        if (cancelled) {
          return;
        }

        dispatch({ type: "set-projects", projects });
        const latestProjectPath = projects[0]?.path;
        if (!latestProjectPath) {
          return;
        }

        await openWorkspaceInternal(latestProjectPath, {
          notifyError: false,
          refreshProjects: false,
        });
      } catch {
        // Ignore restore failures and start from an empty session.
      }
    }

    void restoreWorkspaceSession();

    return () => {
      cancelled = true;
    };
  }, [openWorkspaceInternal]);

  return {
    ...state,
    openWorkspace,
    selectFolder,
    refreshProjects,
    deleteProject,
  };
}
