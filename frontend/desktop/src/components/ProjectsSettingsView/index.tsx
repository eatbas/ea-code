import type { ReactNode } from "react";
import type { ProjectEntry } from "../../types";
import { ProjectSettingsRow } from "./ProjectSettingsRow";

interface ProjectsSettingsViewProps {
  projects: ProjectEntry[];
  onAddProject: () => void;
  onRemoveProject: (projectPath: string) => void;
}

export function ProjectsSettingsView({ projects, onAddProject, onRemoveProject }: ProjectsSettingsViewProps): ReactNode {
  return (
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          {/* Header */}
          <div className="flex items-center justify-between">
            <h1 className="text-xl font-bold text-fg">Select a project</h1>
            <button
              type="button"
              onClick={onAddProject}
              className="rounded-md border border-edge bg-elevated px-4 py-2 text-sm font-medium text-fg transition-colors hover:bg-active hover:text-fg"
            >
              Add project
            </button>
          </div>

          {/* Project list */}
          {projects.length === 0 ? (
            <p className="py-12 text-center text-sm text-fg-muted">
              No projects yet. Click &quot;Add project&quot; to get started.
            </p>
          ) : (
            <div className="flex flex-col gap-3">
              {projects.map((project) => (
                <ProjectSettingsRow
                  key={project.id}
                  project={project}
                  onRemove={onRemoveProject}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
