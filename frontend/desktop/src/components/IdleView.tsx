import type { ReactNode } from "react";
import { useState, useRef, useCallback } from "react";
import { ChevronDown, Folder, FolderPlus } from "lucide-react";
import { useClickOutside } from "../hooks/useClickOutside";
import { useFooterErrorHandler } from "../hooks/useFooterErrorHandler";
import type { WorkspaceInfo, ProjectEntry } from "../types";
import { folderName, projectDisplayName } from "../utils/formatters";
import { Checkmark } from "./shared/Checkmark";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";

interface IdleViewProps {
  workspace: WorkspaceInfo | null;
  projects: ProjectEntry[];
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
}

/** Centred idle landing screen with logo, heading, and workspace selector. */
export function IdleView({
  workspace,
  projects,
  onSelectProject,
  onAddProject,
  onOpenProjectFolder,
  onOpenInVsCode,
}: IdleViewProps): ReactNode {
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const closeDropdown = useCallback(() => setDropdownOpen(false), []);
  useClickOutside(dropdownRef, closeDropdown, dropdownOpen);
  const handleFooterError = useFooterErrorHandler();

  const workspaceEntry = workspace
    ? projects.find((project) => project.path === workspace.path)
    : null;
  const workspaceLabel = workspaceEntry
    ? projectDisplayName(workspaceEntry)
    : workspace ? folderName(workspace.path) : "";

  return (
    <div className="flex h-full flex-col bg-surface">
      <div className="flex flex-1 flex-col items-center justify-center gap-4">
        <img src="/logo.png" alt="Maestro logo" className="h-40 w-40" />

        <h1 className="text-3xl font-bold text-fg">maestro</h1>

        <div ref={dropdownRef} className="relative">
          <button
            onClick={() => setDropdownOpen((prev) => !prev)}
            className="flex items-center gap-2 text-lg text-fg-muted transition-colors hover:text-fg"
          >
            <span>{workspace ? workspaceLabel : "Select a project..."}</span>
            <ChevronDown
              size={16}
              className={`shrink-0 transition-transform ${dropdownOpen ? "rotate-180" : ""}`}
            />
          </button>

          {dropdownOpen && (
            <div className="absolute top-full left-1/2 z-50 mt-2 w-64 -translate-x-1/2 rounded-lg border border-edge bg-panel py-1 shadow-lg">
              {projects.length === 0 && (
                <span className="block px-3 py-2 text-xs text-fg-faint">No projects yet</span>
              )}

              {projects.map((project) => {
                const isActive = project.path === workspace?.path;
                return (
                  <button
                    key={project.path}
                    onClick={() => {
                      void onSelectProject(project.path);
                      setDropdownOpen(false);
                    }}
                    className={`flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors ${
                      isActive
                        ? "bg-elevated text-fg"
                        : "text-fg-muted hover:bg-elevated hover:text-fg"
                    }`}
                    title={project.path}
                  >
                    {isActive ? (
                      <Checkmark size="md" className="h-4 w-4 shrink-0" />
                    ) : (
                      <Folder size={16} className="shrink-0" />
                    )}
                    <span className="truncate">{projectDisplayName(project)}</span>
                  </button>
                );
              })}

              <div className="my-1 border-t border-edge" />
              <button
                onClick={() => {
                  onAddProject();
                  setDropdownOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-sm text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
              >
                <FolderPlus size={16} className="shrink-0" />
                <span>Add project</span>
              </button>
            </div>
          )}
        </div>

        {workspace && (
          <div className="flex gap-2 text-xs text-fg-muted">
            {workspace.isGitRepo && (
              <span className="rounded bg-elevated px-2 py-0.5">
                {workspace.branch ?? "git"}
              </span>
            )}
          </div>
        )}
      </div>

      <div className="mt-auto border-t border-menu-surface px-6 py-4">
        {workspace?.path ? (
          <div className="mx-auto flex w-full max-w-5xl">
            <WorkspaceFooter
              path={workspace.path}
              onOpenProjectFolder={onOpenProjectFolder}
              onOpenInVsCode={onOpenInVsCode}
              onError={handleFooterError}
            />
          </div>
        ) : (
          <div className="mx-auto flex w-full max-w-5xl items-center justify-between text-xs text-fg-muted">
            <span className="truncate" title="No project selected">No project selected</span>
          </div>
        )}
      </div>
    </div>
  );
}
