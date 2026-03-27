import type { ReactNode } from "react";
import { useState, useRef, useCallback } from "react";
import { useClickOutside } from "../hooks/useClickOutside";
import type { WorkspaceInfo, ProjectEntry } from "../types";
import { folderName, projectDisplayName } from "../utils/formatters";
import { Checkmark } from "./shared/Checkmark";
import { useToast } from "./shared/Toast";
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
  const toast = useToast();
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const closeDropdown = useCallback(() => setDropdownOpen(false), []);
  useClickOutside(dropdownRef, closeDropdown, dropdownOpen);

  const workspaceLabel = workspace ? folderName(workspace.path) : "";

  return (
    <div className="flex h-full flex-col bg-[#0b0b0c]">
      <div className="flex flex-1 flex-col items-center justify-center gap-4">
        <img src="/logo.png" alt="EA Code logo" className="h-40 w-40" />

        <h1 className="text-3xl font-bold text-[#f5f5f5]">ea-code</h1>

        <div ref={dropdownRef} className="relative">
          <button
            onClick={() => setDropdownOpen((prev) => !prev)}
            className="flex items-center gap-2 text-lg text-[#8b8b93] transition-colors hover:text-[#f5f5f5]"
          >
            <span>{workspace ? workspaceLabel : "Select a project..."}</span>
            <svg
              className={`h-4 w-4 shrink-0 transition-transform ${dropdownOpen ? "rotate-180" : ""}`}
              xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </button>

          {dropdownOpen && (
            <div className="absolute top-full left-1/2 z-50 mt-2 w-64 -translate-x-1/2 rounded-lg border border-[#313134] bg-[#151516] py-1 shadow-lg">
              {projects.length === 0 && (
                <span className="block px-3 py-2 text-xs text-[#72727a]">No projects yet</span>
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
                        ? "bg-[#202022] text-[#f5f5f5]"
                        : "text-[#8b8b93] hover:bg-[#202022] hover:text-[#f5f5f5]"
                    }`}
                    title={project.path}
                  >
                    {isActive ? (
                      <Checkmark size="md" className="h-4 w-4 shrink-0" />
                    ) : (
                      <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                      </svg>
                    )}
                    <span className="truncate">{projectDisplayName(project)}</span>
                  </button>
                );
              })}

              <div className="my-1 border-t border-[#313134]" />
              <button
                onClick={() => {
                  onAddProject();
                  setDropdownOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-sm text-[#8b8b93] transition-colors hover:bg-[#202022] hover:text-[#f5f5f5]"
              >
                <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                  <line x1="12" y1="11" x2="12" y2="17" />
                  <line x1="9" y1="14" x2="15" y2="14" />
                </svg>
                <span>Add project</span>
              </button>
            </div>
          )}
        </div>

        {workspace && (
          <div className="flex gap-2 text-xs text-[#8b8b93]">
            {workspace.isGitRepo && (
              <span className="rounded bg-[#202022] px-2 py-0.5">
                {workspace.branch ?? "git"}
              </span>
            )}
          </div>
        )}
      </div>

      <div className="mt-auto border-t border-[#232325] px-6 py-4">
        {workspace?.path ? (
          <div className="mx-auto flex w-full max-w-5xl">
            <WorkspaceFooter
              path={workspace.path}
              onOpenProjectFolder={onOpenProjectFolder}
              onOpenInVsCode={onOpenInVsCode}
              onError={() => {
                toast.error("Failed to open project action.");
              }}
            />
          </div>
        ) : (
          <div className="mx-auto flex w-full max-w-5xl items-center justify-between text-xs text-[#8b8b93]">
            <span className="truncate" title="No project selected">No project selected</span>
          </div>
        )}
      </div>
    </div>
  );
}
