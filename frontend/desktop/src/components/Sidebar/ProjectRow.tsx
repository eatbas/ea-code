import type { ReactNode } from "react";
import {
  Archive,
  ChevronDown,
  ChevronUp,
  Ellipsis,
  Eye,
  Folder,
  Pencil,
  SquarePen,
  Upload,
  X,
} from "lucide-react";
import { useSidebarRowMenu } from "../../hooks/useSidebarRowMenu";
import { ConfirmActionPopover } from "../shared/ConfirmActionPopover";
import { InlineRenameInput } from "../shared/InlineRenameInput";
import { RowDropdownMenu, type RowDropdownMenuItem } from "../shared/RowDropdownMenu";

interface ProjectRowProps {
  projectPath: string;
  projectLabel: string;
  isActive: boolean;
  expanded: boolean;
  isArchived: boolean;
  hasConversations: boolean;
  hasRunningConversation: boolean;
  showingArchivedConversations: boolean;
  onProjectClick: () => void;
  onCreateConversation: () => void;
  onToggleShowArchivedConversations?: () => void;
  onRenameProject?: (name: string) => void;
  onArchiveProject?: () => void;
  onUnarchiveProject?: () => void;
  onRemoveProject?: () => void;
}

export function ProjectRow({
  projectPath,
  projectLabel,
  isActive,
  expanded,
  isArchived,
  hasConversations,
  hasRunningConversation,
  showingArchivedConversations,
  onProjectClick,
  onCreateConversation,
  onToggleShowArchivedConversations,
  onRenameProject,
  onArchiveProject,
  onUnarchiveProject,
  onRemoveProject,
}: ProjectRowProps): ReactNode {
  const {
    menuRef,
    renameInputRef,
    menuOpen,
    setMenuOpen,
    renaming,
    setRenaming,
    confirmAction,
    setConfirmAction,
    renameValue,
    setRenameValue,
    busyAction,
  } = useSidebarRowMenu(projectLabel);

  const busy = busyAction !== null;
  const hasActions = Boolean(onRenameProject || onArchiveProject || onUnarchiveProject || onRemoveProject);

  function handleRenameSubmit(): void {
    const trimmed = renameValue.trim();
    if (trimmed && onRenameProject) {
      onRenameProject(trimmed);
    }
    setRenaming(false);
  }

  if (renaming) {
    return (
      <InlineRenameInput
        inputRef={renameInputRef}
        value={renameValue}
        onChange={setRenameValue}
        onSubmit={handleRenameSubmit}
        onCancel={() => { setRenameValue(projectLabel); setRenaming(false); }}
        placeholder="Project name"
      />
    );
  }

  const menuItems: RowDropdownMenuItem[] = [];
  if (onToggleShowArchivedConversations) {
    menuItems.push({
      label: showingArchivedConversations ? "Hide archived" : "Show archived",
      icon: Eye,
      onClick: () => { setMenuOpen(false); onToggleShowArchivedConversations(); },
    });
  }
  if (onRenameProject) {
    menuItems.push({ label: "Rename project", icon: Pencil, onClick: () => { setMenuOpen(false); setRenaming(true); } });
  }
  if (!isArchived && onArchiveProject) {
    menuItems.push({ label: "Archive project", icon: Archive, onClick: () => { setMenuOpen(false); setConfirmAction("archive"); } });
  }
  if (isArchived && onUnarchiveProject) {
    menuItems.push({ label: "Unarchive project", icon: Upload, onClick: () => { setMenuOpen(false); onUnarchiveProject(); } });
  }
  if (onRemoveProject) {
    menuItems.push({ label: "Remove project", icon: X, danger: true, onClick: () => { setMenuOpen(false); setConfirmAction("remove"); } });
  }

  return (
    <div className="group/project relative">
      <button
        type="button"
        onClick={onProjectClick}
        className={`relative mx-1 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-lg py-1.5 pr-16 text-left text-sm transition-colors ${
          hasRunningConversation ? "pl-9" : "pl-3"
        } ${
          isActive
            ? "bg-elevated text-fg"
            : isArchived
              ? "text-fg-faint hover:bg-elevated hover:text-fg-muted"
              : "text-fg-muted hover:bg-elevated hover:text-fg"
        }`}
        title={projectPath}
      >
        {hasRunningConversation && (
          <span
            className="absolute left-3 top-1/2 inline-flex h-2.5 w-2.5 -translate-y-1/2 animate-pulse rounded-full bg-running-dot shadow-[0_0_0_3px_rgba(30,183,95,0.16)]"
            title="A conversation is running in this project"
          />
        )}
        <span className="relative h-4 w-4 shrink-0">
          <Folder
            size={16}
            className={`absolute inset-0 transition-opacity ${hasConversations ? "opacity-100 group-hover/project:opacity-0" : "opacity-100"}`}
          />
          {hasConversations && (
            expanded
              ? <ChevronUp size={16} className="absolute inset-0 opacity-0 transition-opacity group-hover/project:opacity-100" />
              : <ChevronDown size={16} className="absolute inset-0 opacity-0 transition-opacity group-hover/project:opacity-100" />
          )}
        </span>
        <span className="flex min-w-0 items-center gap-2">
          <span className="truncate font-medium">{projectLabel}</span>
          {isArchived && (
            <span className="rounded-full border border-edge px-1.5 py-0.5 text-[10px] uppercase tracking-[0.08em] text-fg-faint">Archived</span>
          )}
        </span>
      </button>

      <div ref={menuRef} className="absolute right-2 top-2 z-10 flex items-start gap-1">
        {confirmAction ? (
          <div className="absolute right-0 top-1/2 z-20 -translate-y-1/2">
            <ConfirmActionPopover
              label={confirmAction === "archive" ? "Archive?" : "Delete?"}
              confirmLabel={confirmAction === "archive" ? "Archive" : "Delete"}
              onConfirm={() => {
                if (confirmAction === "archive") {
                  onArchiveProject?.();
                } else {
                  onRemoveProject?.();
                }
                setConfirmAction(null);
              }}
              onCancel={() => setConfirmAction(null)}
              disabled={busy}
            />
          </div>
        ) : hasActions && (
          <>
            <button
              type="button"
              onClick={(event) => { event.stopPropagation(); setMenuOpen((current) => !current); }}
              className={`rounded p-1 text-fg-faint transition-opacity hover:bg-active hover:text-fg ${menuOpen ? "opacity-100" : "opacity-0 group-hover/project:opacity-100"}`}
              title="Project actions"
            >
              <Ellipsis size={12} />
            </button>
            {menuOpen && <RowDropdownMenu items={menuItems} />}
          </>
        )}
        <button
          type="button"
          onClick={(event) => { event.stopPropagation(); onCreateConversation(); }}
          className="rounded p-1 text-fg-faint opacity-0 transition-colors transition-opacity hover:bg-new-btn-bg-hover hover:text-new-btn-text group-hover/project:opacity-100"
          title="New conversation"
        >
          <SquarePen size={13} strokeWidth={2} />
        </button>
      </div>
    </div>
  );
}
