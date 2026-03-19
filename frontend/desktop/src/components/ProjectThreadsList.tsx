import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import type { ProjectEntry, SessionMeta } from "../types";
import {
  formatCompactTimestamp,
  formatTimestamp,
  parseUtcTimestamp,
  projectDisplayName,
} from "../utils/formatters";
import { isActiveStatusValue } from "../utils/statusHelpers";

interface ProjectThreadsListProps {
  projects: ProjectEntry[];
  sessions: SessionMeta[];
  activeProjectPath?: string;
  activeSessionId?: string;
  /** Session ID of the currently running pipeline (shows spinner). */
  runningSessionId?: string;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onSelectSession: (sessionId: string) => void;
  onArchiveSession?: (sessionId: string) => void;
}

/** Renders projects with nested sessions for the active project. */
export function ProjectThreadsList({
  projects,
  sessions,
  activeProjectPath,
  activeSessionId,
  runningSessionId,
  onSelectProject,
  onSelectSession,
  onArchiveSession,
}: ProjectThreadsListProps): ReactNode {
  const [confirmingId, setConfirmingId] = useState<string | null>(null);

  /** Sessions sorted most-recent first by updatedAt. */
  const sortedSessions = useMemo(
    () =>
      [...sessions].sort(
        (a, b) =>
          parseUtcTimestamp(b.updatedAt).getTime() -
          parseUtcTimestamp(a.updatedAt).getTime(),
      ),
    [sessions],
  );

  function handleArchiveClick(e: React.MouseEvent, sessionId: string): void {
    e.stopPropagation();
    setConfirmingId(sessionId);
  }

  function handleConfirm(e: React.MouseEvent, sessionId: string): void {
    e.stopPropagation();
    onArchiveSession?.(sessionId);
    setConfirmingId(null);
  }

  function handleCancel(e: React.MouseEvent): void {
    e.stopPropagation();
    setConfirmingId(null);
  }

  return (
    <div className="app-scrollbar min-h-0 flex-1 overflow-x-hidden overflow-y-auto [scrollbar-gutter:stable_both-edges]">
      <div className="px-2">
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
              <div key={project.path} className="rounded-lg">
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
                  <span className="truncate">{projectDisplayName(project)}</span>
                </button>

                {isActiveProject && (
                  <div className="ml-5 mt-1 border-l border-[#2e2e48] pl-2">
                    {sessions.length === 0 ? (
                      <p className="px-3 py-2 text-xs text-[#6f7086]">No threads</p>
                    ) : (
                      <div className="flex flex-col gap-0.5 py-1">
                        {sortedSessions.map((session) => {
                          const isActiveSession = session.id === activeSessionId;
                          const isRunningSession =
                            session.id === runningSessionId ||
                            isActiveStatusValue(session.lastStatus);
                          const isConfirming = confirmingId === session.id;

                          if (isConfirming) {
                            return (
                              <div
                                key={session.id}
                                className="flex items-center gap-1 rounded-md bg-[#24243a] px-3 py-1.5"
                              >
                                <span className="flex-1 truncate text-xs text-[#e4e4ed]">Delete?</span>
                                <button
                                  onClick={(e) => handleConfirm(e, session.id)}
                                  className="rounded px-1.5 py-0.5 text-[10px] font-medium text-[#ef4444] hover:bg-[#ef4444]/20 transition-colors"
                                >
                                  Yes
                                </button>
                                <button
                                  onClick={handleCancel}
                                  className="rounded px-1.5 py-0.5 text-[10px] font-medium text-[#9898b0] hover:bg-[#2e2e48] transition-colors"
                                >
                                  No
                                </button>
                              </div>
                            );
                          }

                          return (
                            <div
                              key={session.id}
                              className={`group flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left text-sm transition-colors ${
                                isActiveSession
                                  ? "bg-[#24243a] text-[#e4e4ed]"
                                  : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                              }`}
                            >
                              {isRunningSession && (
                                <svg className="h-3 w-3 shrink-0 animate-spin text-[#22c55e]" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                                </svg>
                              )}
                              <button
                                onClick={() => onSelectSession(session.id)}
                                className="flex-1 truncate text-left"
                              >
                                {session.lastPrompt ?? session.title}
                              </button>
                              {onArchiveSession ? (
                                <>
                                  <span
                                    className="shrink-0 text-[10px] text-[#6f7086] group-hover:hidden"
                                    title={`Started ${formatTimestamp(session.createdAt)}`}
                                  >
                                    {formatCompactTimestamp(session.createdAt)}
                                  </span>
                                  <button
                                    onClick={(e) => handleArchiveClick(e, session.id)}
                                    className="hidden shrink-0 rounded p-0.5 text-[#6f7086] hover:text-[#ef4444] group-hover:block transition-all"
                                    title="Archive session"
                                  >
                                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                      <polyline points="3 6 5 6 21 6" />
                                      <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                                      <path d="M10 11v6" />
                                      <path d="M14 11v6" />
                                    </svg>
                                  </button>
                                </>
                              ) : (
                                <span
                                  className="shrink-0 text-[10px] text-[#6f7086]"
                                  title={`Started ${formatTimestamp(session.createdAt)}`}
                                >
                                  {formatCompactTimestamp(session.createdAt)}
                                </span>
                              )}
                            </div>
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
    </div>
  );
}
