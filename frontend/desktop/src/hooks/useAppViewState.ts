import { useCallback, useState } from "react";
import type { ActiveView, WorkspaceInfo } from "../types";

interface UseAppViewStateArgs {
  workspace: WorkspaceInfo | null;
  openWorkspace: (path: string) => Promise<void>;
}

interface UseAppViewStateReturn {
  activeView: ActiveView;
  setActiveView: (view: ActiveView) => void;
  handleSelectProject: (projectPath: string) => Promise<void>;
}

/** Owns App-level view state. */
export function useAppViewState({
  openWorkspace,
}: UseAppViewStateArgs): UseAppViewStateReturn {
  const [activeView, setActiveView] = useState<ActiveView>("home");

  const handleSelectProject = useCallback(async (projectPath: string): Promise<void> => {
    await openWorkspace(projectPath);
    setActiveView("home");
  }, [openWorkspace]);

  return {
    activeView,
    setActiveView,
    handleSelectProject,
  };
}
