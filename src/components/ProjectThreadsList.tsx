import type { ReactNode } from "react";
import type { ProjectSummary, SessionSummary } from "../types";

interface ProjectThreadsListProps {
  projects: ProjectSummary[];
  sessions: SessionSummary[];
  activeProjectPath?: string;
  activeSessionId?: string;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onSelectSession: (sessionId: string) => void;
}

function getProjectName(project: ProjectSummary): string {
  if (project.name.trim().length > 0) {
    return project.name;
  }

  const parts = project.path.split(/[/\\]+/);
  return parts[parts.length - 1] || project.path;
}

/** Renders projects with nested sessions for the active project. */
export function ProjectThreadsList({
  projects,
  sessions,
  activeProjectPath,
  activeSessionId,
  onSelectProject,
  onSelectSession,
}: ProjectThreadsListProps): ReactNode {
  return (
    <div className="flex-1 overflow-y-auto px-2">
      <div className="mb-2 px-2 pt-1">
        <span className="text-xs font-medium uppercase tracking-wide text-[#6f7086]">Sessions</span>
      </div>

      {projects.length === 0 && (
        <p className="px-3 py-4 text-xs text-[#9898b0]">No projects yet</p>
      )}

      <div className="flex flex-col gap-1 pb-3">
        {projects.map((project) => {
          const isActiveProject = project.path === activeProjectPath;
          return (
            <div key={project.id} className="rounded-lg">
              <button
                onClick={() => void onSelectProject(project.path)}
                className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                  isActiveProject
                    ? "text-[#e4e4ed]"
                    : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                }`}
                title={project.path}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                </svg>
                <span className="truncate">{getProjectName(project)}</span>
              </button>

              {isActiveProject && (
                <div className="ml-5 mt-1 border-l border-[#2e2e48] pl-2">
                  {sessions.length === 0 ? (
                    <p className="px-3 py-2 text-xs text-[#6f7086]">No threads</p>
                  ) : (
                    <div className="flex flex-col gap-0.5 py-1">
                      {sessions.map((session) => {
                        const isActiveSession = session.id === activeSessionId;
                        return (
                          <button
                            key={session.id}
                            onClick={() => onSelectSession(session.id)}
                            className={`flex w-full items-center justify-between gap-2 rounded-md px-3 py-1.5 text-left text-sm transition-colors ${
                              isActiveSession
                                ? "bg-[#24243a] text-[#e4e4ed]"
                                : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                            }`}
                          >
                            <span className="truncate">{session.lastPrompt ?? session.title}</span>
                            <span className="shrink-0 text-xs text-[#6f7086]">{session.runCount}</span>
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
