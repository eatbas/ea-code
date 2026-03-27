import type { ReactNode } from "react";
import type { ActiveView, WorkspaceInfo, ProjectEntry } from "../types";
import { IdleView } from "./IdleView";
import { CliSetupRoute } from "./CliSetupView/Route";

interface AppContentRouterProps {
  activeView: ActiveView;
  workspace: WorkspaceInfo | null;
  projects: ProjectEntry[];
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
}

/** Routes the main content panel between home and settings views. */
export function AppContentRouter({
  activeView,
  workspace,
  projects,
  onSelectProject,
  onAddProject,
  onOpenProjectFolder,
  onOpenInVsCode,
}: AppContentRouterProps): ReactNode {
  if (activeView === "cli-setup") {
    return <CliSetupRoute />;
  }

  return (
    <IdleView
      workspace={workspace}
      projects={projects}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
      onOpenProjectFolder={onOpenProjectFolder}
      onOpenInVsCode={onOpenInVsCode}
    />
  );
}
