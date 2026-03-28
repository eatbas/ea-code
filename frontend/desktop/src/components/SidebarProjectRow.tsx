import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useClickOutside } from "../hooks/useClickOutside";

interface SidebarProjectRowProps {
  projectPath: string;
  projectLabel: string;
  isActive: boolean;
  expanded: boolean;
  hasConversations: boolean;
  hasRunningConversation: boolean;
  onProjectClick: () => void;
  onCreateConversation: () => void;
  onRenameProject?: (name: string) => void;
  onArchiveProject?: () => void;
  onRemoveProject?: () => void;
}

export function SidebarProjectRow({
  projectPath,
  projectLabel,
  isActive,
  expanded,
  hasConversations,
  hasRunningConversation,
  onProjectClick,
  onCreateConversation,
  onRenameProject,
  onArchiveProject,
  onRemoveProject,
}: SidebarProjectRowProps): ReactNode {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const [menuOpen, setMenuOpen] = useState<boolean>(false);
  const [renaming, setRenaming] = useState<boolean>(false);
  const [confirmAction, setConfirmAction] = useState<"archive" | "remove" | null>(null);
  const [renameValue, setRenameValue] = useState<string>(projectLabel);

  useClickOutside(menuRef, () => setMenuOpen(false), menuOpen);

  useEffect(() => {
    if (!renaming) {
      setRenameValue(projectLabel);
    }
  }, [projectLabel, renaming]);

  useEffect(() => {
    if (renaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renaming]);

  if (renaming) {
    return (
      <div className="rounded-lg border border-edge bg-[#19191a] px-3 py-3">
        <input
          ref={renameInputRef}
          type="text"
          value={renameValue}
          onChange={(event) => setRenameValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              const trimmed = renameValue.trim();
              if (trimmed && onRenameProject) {
                onRenameProject(trimmed);
              }
              setRenaming(false);
            }
            if (event.key === "Escape") {
              event.preventDefault();
              setRenameValue(projectLabel);
              setRenaming(false);
            }
          }}
          className="w-full rounded-md border border-edge bg-panel px-2 py-1.5 text-sm text-fg outline-none transition-colors focus:border-[#5a5a61]"
          placeholder="Project name"
        />
        <div className="mt-2 flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={() => {
              setRenameValue(projectLabel);
              setRenaming(false);
            }}
            className="rounded px-2 py-1 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              const trimmed = renameValue.trim();
              if (trimmed && onRenameProject) {
                onRenameProject(trimmed);
              }
              setRenaming(false);
            }}
            className="rounded bg-elevated px-2 py-1 text-[11px] font-medium text-fg transition-colors hover:bg-[#2a2a2d]"
          >
            Save
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="group/project relative">
      <button
        type="button"
        onClick={onProjectClick}
        className={`relative mx-1 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-lg py-1.5 pr-16 pl-3 text-left text-sm transition-colors ${
          isActive
            ? "bg-elevated text-fg"
            : "text-fg-muted hover:bg-elevated hover:text-fg"
        }`}
        title={projectPath}
      >
        <span className="relative h-4 w-4 shrink-0">
          <svg
            className={`absolute inset-0 h-4 w-4 transition-opacity ${
              hasConversations ? "opacity-100 group-hover/project:opacity-0" : "opacity-100"
            }`}
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          </svg>
          {hasConversations && (
            <svg
              className="absolute inset-0 h-4 w-4 opacity-0 transition-opacity group-hover/project:opacity-100"
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {expanded ? (
                <polyline points="18 15 12 9 6 15" />
              ) : (
                <polyline points="6 9 12 15 18 9" />
              )}
            </svg>
          )}
        </span>
        <span className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium">{projectLabel}</span>
          {hasRunningConversation && (
            <span
              className="inline-flex h-2.5 w-2.5 shrink-0 rounded-full bg-[#1eb75f] shadow-[0_0_0_3px_rgba(30,183,95,0.16)] animate-pulse"
              title="A conversation is running in this project"
            />
          )}
        </span>
      </button>

      <div ref={menuRef} className="absolute top-2 right-2 z-10 flex items-start gap-1">
        {confirmAction ? (
          <div className="absolute top-1/2 right-0 z-20 flex -translate-y-1/2 items-center gap-1 rounded-lg border border-edge bg-[#232325] px-1.5 py-1 shadow-[0_10px_24px_rgba(0,0,0,0.28)]">
            <span className="px-1 text-[11px] text-fg-muted">
              {confirmAction === "archive" ? "Archive?" : "Delete?"}
            </span>
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                setConfirmAction(null);
              }}
              className="rounded px-2 py-1 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                if (confirmAction === "archive") {
                  onArchiveProject?.();
                } else {
                  onRemoveProject?.();
                }
                setConfirmAction(null);
              }}
              className="rounded bg-[#3a1418] px-2 py-1 text-[11px] font-medium text-[#ffb4bb] transition-colors hover:bg-[#521a21] hover:text-[#ffd7dc]"
            >
              {confirmAction === "archive" ? "Archive" : "Delete"}
            </button>
          </div>
        ) : (onRenameProject || onArchiveProject || onRemoveProject) && (
          <>
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                setMenuOpen((current) => !current);
              }}
              className={`rounded p-1 text-fg-faint transition-opacity hover:bg-active hover:text-fg ${
                menuOpen ? "opacity-100" : "opacity-0 group-hover/project:opacity-100"
              }`}
              title="Project actions"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <circle cx="5" cy="12" r="1.8" />
                <circle cx="12" cy="12" r="1.8" />
                <circle cx="19" cy="12" r="1.8" />
              </svg>
            </button>

            {menuOpen && (
              <div className="absolute top-full right-0 z-20 mt-2 min-w-40 rounded-xl border border-edge bg-[#232325] p-1 shadow-[0_14px_30px_rgba(0,0,0,0.35)]">
                {onRenameProject && (
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      setMenuOpen(false);
                      setRenaming(true);
                    }}
                    className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-fg transition-colors hover:bg-elevated"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M12 20h9" />
                      <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
                    </svg>
                    Rename project
                  </button>
                )}
                {onArchiveProject && (
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      setMenuOpen(false);
                      setConfirmAction("archive");
                    }}
                    className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-fg transition-colors hover:bg-elevated"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <rect x="3" y="4" width="18" height="4" rx="1" />
                      <path d="M5 8h14v10a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2Z" />
                    </svg>
                    Archive project
                  </button>
                )}
                {onRemoveProject && (
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    setMenuOpen(false);
                    setConfirmAction("remove");
                  }}
                  className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-[#ffb4bb] transition-colors hover:bg-[#3a1418] hover:text-[#ffd7dc]"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18" />
                    <line x1="6" y1="6" x2="18" y2="18" />
                  </svg>
                  Remove project
                </button>
                )}
              </div>
            )}
          </>
        )}
        <button
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            onCreateConversation();
          }}
          className="rounded p-1 text-fg-faint opacity-0 transition-colors transition-opacity hover:bg-[#18210f] hover:text-[#7ee787] group-hover/project:opacity-100"
          title="New conversation"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18 2l4 4-10 10-4 1 1-4Z" />
          </svg>
        </button>
      </div>
    </div>
  );
}
