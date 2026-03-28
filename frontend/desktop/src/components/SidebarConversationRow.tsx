import type { ReactNode } from "react";
import { Archive, Ellipsis, Pencil, Pin, Upload, X } from "lucide-react";
import type { ConversationSummary } from "../types";
import { formatRelativeTime } from "../utils/formatters";
import { useSidebarRowMenu } from "../hooks/useSidebarRowMenu";
import { InlineRenameInput } from "./shared/InlineRenameInput";
import { ConfirmActionPopover } from "./shared/ConfirmActionPopover";
import { RowDropdownMenu, type RowDropdownMenuItem } from "./shared/RowDropdownMenu";

interface SidebarConversationRowProps {
  conversation: ConversationSummary;
  isActive: boolean;
  projectPath: string;
  onSelectConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onRenameConversation: (
    projectPath: string,
    conversationId: string,
    title: string,
  ) => void | Promise<void>;
  onArchiveConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onUnarchiveConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onRemoveConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onSetConversationPinned: (
    projectPath: string,
    conversationId: string,
    pinned: boolean,
  ) => void | Promise<void>;
}

export function SidebarConversationRow({
  conversation,
  isActive,
  projectPath,
  onSelectConversation,
  onRenameConversation,
  onArchiveConversation,
  onUnarchiveConversation,
  onRemoveConversation,
  onSetConversationPinned,
}: SidebarConversationRowProps): ReactNode {
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
    withBusy,
  } = useSidebarRowMenu(conversation.title);

  const busy = busyAction !== null;
  const isArchived = Boolean(conversation.archivedAt);
  const isRunning = conversation.status === "running";
  const showPinnedMarker = Boolean(conversation.pinnedAt) && !isRunning;

  async function handleRenameSubmit(): Promise<void> {
    const trimmed = renameValue.trim();
    if (!trimmed || trimmed === conversation.title) {
      setRenameValue(conversation.title);
      setRenaming(false);
      return;
    }
    await withBusy("rename", async () => {
      await onRenameConversation(projectPath, conversation.id, trimmed);
      setRenaming(false);
    });
  }

  if (renaming) {
    return (
      <InlineRenameInput
        inputRef={renameInputRef}
        value={renameValue}
        onChange={setRenameValue}
        onSubmit={() => { void handleRenameSubmit(); }}
        onCancel={() => { setRenameValue(conversation.title); setRenaming(false); }}
        placeholder="Conversation name"
        disabled={busy}
      />
    );
  }

  const menuItems: RowDropdownMenuItem[] = [
    { label: "Rename", icon: Pencil, onClick: () => { setMenuOpen(false); setRenaming(true); }, disabled: busy },
    ...(isArchived ? [{
      label: "Unarchive", icon: Upload, disabled: busy,
      onClick: () => { void withBusy("unarchive", () => onUnarchiveConversation(projectPath, conversation.id) as Promise<void>).then(() => setMenuOpen(false)); },
    }] : []),
    { label: "Remove thread", icon: X, danger: true, disabled: busy,
      onClick: () => { setMenuOpen(false); setConfirmAction("remove"); },
    },
  ];

  return (
    <div className="group/conversation relative">
      <div className={`absolute top-1/2 left-3 z-[1] h-4 -translate-y-1/2 ${isRunning ? "w-7" : "w-4"}`}>
        {isRunning && (
          <span
            className="pointer-events-none absolute top-1/2 left-0 inline-flex h-2.5 w-2.5 -translate-y-1/2 rounded-full bg-running-dot shadow-[0_0_0_3px_rgba(30,183,95,0.16)] animate-pulse"
            title="Conversation running"
          />
        )}
        <button
          type="button"
          onClick={(event) => { event.stopPropagation(); void withBusy("pin", () => onSetConversationPinned(projectPath, conversation.id, !conversation.pinnedAt) as Promise<void>); }}
          className={`absolute top-0 flex h-4 w-4 items-center justify-center rounded transition-opacity hover:bg-active ${
            isRunning ? "left-3" : "left-0"
          } ${
            showPinnedMarker
              ? "text-new-btn-text opacity-100 hover:text-success-chip-text"
              : isRunning && conversation.pinnedAt
                ? "text-new-btn-text opacity-0 group-hover/conversation:opacity-100 hover:text-success-chip-text"
                : "text-fg-faint opacity-0 hover:text-fg group-hover/conversation:opacity-100"
          }`}
          title={conversation.pinnedAt ? "Unpin conversation" : "Pin conversation"}
          disabled={busy}
        >
          <Pin size={11} strokeWidth={2} className="-rotate-[28deg]" fill={showPinnedMarker ? "currentColor" : "none"} />
        </button>
      </div>

      <button
        type="button"
        onClick={() => { void onSelectConversation(projectPath, conversation.id); }}
        className={`relative mx-1 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-lg py-1.5 ${isRunning ? "pl-11" : "pl-9"} text-left transition-[padding,color,background-color] ${
          menuOpen ? "pr-[3.75rem]" : "pr-3 group-hover/conversation:pr-[3.75rem]"
        } ${
          isActive
            ? "bg-row-active text-fg"
            : isArchived
              ? "text-fg-faint hover:bg-row-hover hover:text-fg-muted"
              : "text-fg-inactive hover:bg-row-hover hover:text-fg"
        }`}
      >
        <span className="min-w-0 flex-1 truncate text-sm">{conversation.title}</span>
        <span className={`flex shrink-0 items-center gap-2 text-xs text-fg-subtle transition-opacity duration-150 ${confirmAction ? "opacity-0" : "opacity-100"}`}>
          {isArchived && (
            <span className="rounded-full border border-edge px-1.5 py-0.5 text-[10px] uppercase tracking-[0.08em] text-fg-faint">Archived</span>
          )}
          <span>{formatRelativeTime(conversation.updatedAt)}</span>
        </span>
      </button>

      {confirmAction ? (
        <div className="absolute top-1/2 right-2 z-20 -translate-y-1/2">
          <ConfirmActionPopover
            label={confirmAction === "archive" ? "Archive?" : "Delete?"}
            confirmLabel={confirmAction === "archive" ? "Archive" : "Delete"}
            onConfirm={() => {
              if (confirmAction === "archive") {
                void withBusy("archive", async () => { await onArchiveConversation(projectPath, conversation.id); setConfirmAction(null); setMenuOpen(false); });
                return;
              }
              void withBusy("remove", async () => { await onRemoveConversation(projectPath, conversation.id); setConfirmAction(null); setMenuOpen(false); });
            }}
            onCancel={() => setConfirmAction(null)}
            disabled={busy}
          />
        </div>
      ) : (
        <div ref={menuRef} className="absolute top-2 right-2 z-10 flex items-start gap-1">
          <button
            type="button"
            onClick={(event) => { event.stopPropagation(); setMenuOpen((c) => !c); }}
            className={`rounded p-1 text-fg-faint transition-opacity hover:bg-active hover:text-fg ${menuOpen ? "opacity-100" : "opacity-0 group-hover/conversation:opacity-100"}`}
            title="Thread actions"
          >
            <Ellipsis size={12} />
          </button>
          {!isArchived && (
            <button
              type="button"
              onClick={(event) => { event.stopPropagation(); setMenuOpen(false); setConfirmAction("archive"); }}
              className="rounded p-1 text-fg-faint opacity-0 transition-colors transition-opacity hover:bg-danger-bg hover:text-danger-text group-hover/conversation:opacity-100"
              title="Archive thread"
            >
              <Archive size={12} strokeWidth={2} />
            </button>
          )}
          {menuOpen && <RowDropdownMenu items={menuItems} />}
        </div>
      )}
    </div>
  );
}
