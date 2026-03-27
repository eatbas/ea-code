import type { ReactNode } from "react";
import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useClickOutside } from "../hooks/useClickOutside";
import type { WorkspaceInfo, ProjectEntry } from "../types";
import { folderName, projectDisplayName } from "../utils/formatters";
import { Checkmark } from "./shared/Checkmark";
import { useToast } from "./shared/Toast";

interface IdleViewProps {
  workspace: WorkspaceInfo | null;
  workspacePath?: string;
  projects: ProjectEntry[];
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
}

/** Centred idle landing screen with logo, heading, and workspace selector. */
export function IdleView({
  workspace,
  workspacePath,
  projects,
  onSelectProject,
  onAddProject,
}: IdleViewProps): ReactNode {
  const toast = useToast();
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const closeDropdown = useCallback(() => setDropdownOpen(false), []);
  useClickOutside(dropdownRef, closeDropdown, dropdownOpen);

  const workspaceLabel = workspace ? folderName(workspace.path) : "";

  return (
    <div className="flex h-full flex-col items-center bg-[#0f0f14]">
      {/* Centre content — logo, heading, workspace */}
      <div className="flex flex-1 flex-col items-center justify-center gap-4">
        {/* Logo icon */}
        <img src="/logo.png" alt="EA Code logo" className="h-40 w-40" />

        {/* Heading */}
        <h1 className="text-3xl font-bold text-[#e4e4ed]">ea-code</h1>

        {/* Workspace selector dropdown */}
        <div ref={dropdownRef} className="relative">
          <button
            onClick={() => setDropdownOpen((prev) => !prev)}
            className="flex items-center gap-2 text-lg text-[#9898b0] hover:text-[#e4e4ed] transition-colors"
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
            <div className="absolute top-full left-1/2 z-50 mt-2 w-64 -translate-x-1/2 rounded-lg border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
              {projects.length === 0 && (
                <span className="block px-3 py-2 text-xs text-[#6b6b80]">No projects yet</span>
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
                        ? "bg-[#24243a] text-[#e4e4ed]"
                        : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
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

              {/* Divider + Add project */}
              <div className="my-1 border-t border-[#2e2e48]" />
              <button
                onClick={() => {
                  onAddProject();
                  setDropdownOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
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

        {/* Git badge */}
        {workspace && (
          <div className="flex gap-2 text-xs text-[#9898b0]">
            {workspace.isGitRepo && (
              <span className="rounded bg-[#24243a] px-2 py-0.5">
                {workspace.branch ?? "git"}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Bottom section — workspace path + VS Code */}
      <div className="flex w-full max-w-2xl flex-col gap-2 px-6 pb-8">
        <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
          <span className="truncate" title={workspacePath ?? "No project selected"}>
            {workspacePath ?? "No project selected"}
          </span>
          {workspacePath && (
            <button
              onClick={() => {
                void invoke("open_in_vscode", { path: workspacePath }).catch(() => {
                  toast.error("Failed to open VS Code.");
                });
              }}
              className="ml-4 flex shrink-0 items-center gap-1.5 rounded px-2 py-0.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
              title="Open in VS Code"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M16 3l5 3v12l-5 3L2 12l5-3" />
                <path d="M16 3L7 12l9 9" />
                <path d="M16 3v18" />
              </svg>
              Open in VS Code
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
