import type { ReactNode } from "react";
import type { ProjectEntry } from "../../types";
import { ProjectsSettingsView } from ".";

interface ProjectsSettingsRouteProps {
  projects: ProjectEntry[];
  onAddProject: () => void;
  onRemoveProject: (projectPath: string) => void;
}

export function ProjectsSettingsRoute({ projects, onAddProject, onRemoveProject }: ProjectsSettingsRouteProps): ReactNode {
  return (
    <ProjectsSettingsView
      projects={projects}
      onAddProject={onAddProject}
      onRemoveProject={onRemoveProject}
    />
  );
}
