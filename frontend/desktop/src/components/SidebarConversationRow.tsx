import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import type { ConversationSummary } from "../types";
import { useClickOutside } from "../hooks/useClickOutside";
import { formatRelativeTime } from "../utils/formatters";

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
  onRemoveConversation,
  onSetConversationPinned,
}: SidebarConversationRowProps): ReactNode {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const [menuOpen, setMenuOpen] = useState<boolean>(false);
  const [renaming, setRenaming] = useState<boolean>(false);
  const [confirmAction, setConfirmAction] = useState<"archive" | "remove" | null>(null);
  const [renameValue, setRenameValue] = useState<string>(conversation.title);
  const [busyAction, setBusyAction] = useState<"rename" | "archive" | "remove" | "pin" | null>(null);

  useClickOutside(menuRef, () => setMenuOpen(false), menuOpen);

  useEffect(() => {
    if (!renaming) {
      setRenameValue(conversation.title);
    }
  }, [conversation.title, renaming]);

  useEffect(() => {
    if (renaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renaming]);

  async function handleRenameSubmit(): Promise<void> {
    const trimmed = renameValue.trim();
    if (!trimmed || trimmed === conversation.title) {
      setRenameValue(conversation.title);
      setRenaming(false);
      return;
    }

    setBusyAction("rename");
    try {
      await onRenameConversation(projectPath, conversation.id, trimmed);
      setRenaming(false);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleArchive(): Promise<void> {
    setBusyAction("archive");
    try {
      await onArchiveConversation(projectPath, conversation.id);
      setConfirmAction(null);
      setMenuOpen(false);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleRemove(): Promise<void> {
    setBusyAction("remove");
    try {
      await onRemoveConversation(projectPath, conversation.id);
      setConfirmAction(null);
      setMenuOpen(false);
    } finally {
      setBusyAction(null);
    }
  }

  async function handleTogglePin(): Promise<void> {
    setBusyAction("pin");
    try {
      await onSetConversationPinned(projectPath, conversation.id, !conversation.pinnedAt);
    } finally {
      setBusyAction(null);
    }
  }

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
              void handleRenameSubmit();
            }
            if (event.key === "Escape") {
              event.preventDefault();
              setRenameValue(conversation.title);
              setRenaming(false);
            }
          }}
          className="w-full rounded-md border border-edge bg-panel px-2 py-1.5 text-sm text-fg outline-none transition-colors focus:border-[#5a5a61]"
          placeholder="Conversation name"
          disabled={busyAction !== null}
        />
        <div className="mt-2 flex items-center justify-end gap-2">
          <button
            type="button"
            onClick={() => {
              setRenameValue(conversation.title);
              setRenaming(false);
            }}
            className="rounded px-2 py-1 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
            disabled={busyAction !== null}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              void handleRenameSubmit();
            }}
            className="rounded bg-elevated px-2 py-1 text-[11px] font-medium text-fg transition-colors hover:bg-[#2a2a2d]"
            disabled={busyAction !== null}
          >
            Save
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="group/conversation relative">
      <div className="absolute top-1/2 left-3 z-[1] h-4 w-4 -translate-y-1/2">
        <button
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            void handleTogglePin();
          }}
          className={`flex h-4 w-4 items-center justify-center rounded transition-opacity hover:bg-active ${
            conversation.pinnedAt
              ? "text-[#f3d36d] opacity-100 hover:text-[#ffe39a]"
              : "text-fg-faint opacity-0 hover:text-fg group-hover/conversation:opacity-100"
          }`}
          title={conversation.pinnedAt ? "Unpin thread" : "Pin thread"}
          disabled={busyAction !== null}
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill={conversation.pinnedAt ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 17v5" />
            <path d="M8 3h8l-1 6 3 3H6l3-3-1-6Z" />
          </svg>
        </button>
        {conversation.status === "running" && (
          <span
            className="absolute right-0 bottom-0 inline-flex h-2.5 w-2.5 rounded-full bg-[#1eb75f] shadow-[0_0_0_3px_rgba(30,183,95,0.16)] animate-pulse"
            title="Conversation running"
          />
        )}
      </div>

      <button
        type="button"
        onClick={() => {
          void onSelectConversation(projectPath, conversation.id);
        }}
        className={`relative mx-1 flex w-[calc(100%-0.5rem)] items-center gap-2 rounded-lg py-1.5 pl-9 text-left transition-[padding,color,background-color] ${
          menuOpen ? "pr-[3.75rem]" : "pr-3 group-hover/conversation:pr-[3.75rem]"
        } ${
          isActive
            ? "bg-[#252527] text-fg"
            : "text-[#a3a3aa] hover:bg-[#1d1d1f] hover:text-fg"
        }`}
      >
        <span className="min-w-0 flex-1 truncate text-sm">
          {conversation.title}
        </span>
        <span className={`shrink-0 text-xs text-fg-subtle transition-opacity duration-150 ${
          confirmAction ? "opacity-0" : "opacity-100"
        }`}>
          {formatRelativeTime(conversation.updatedAt)}
        </span>
      </button>

      {confirmAction ? (
        <div className="absolute top-1/2 right-2 z-20 flex -translate-y-1/2 items-center gap-1 rounded-lg border border-edge bg-[#232325] px-1.5 py-1 shadow-[0_10px_24px_rgba(0,0,0,0.28)]">
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
            disabled={busyAction !== null}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              if (confirmAction === "archive") {
                void handleArchive();
                return;
              }
              void handleRemove();
            }}
            className="rounded bg-[#3a1418] px-2 py-1 text-[11px] font-medium text-[#ffb4bb] transition-colors hover:bg-[#521a21] hover:text-[#ffd7dc]"
            disabled={busyAction !== null}
          >
            {confirmAction === "archive" ? "Archive" : "Delete"}
          </button>
        </div>
      ) : (
        <div
          ref={menuRef}
          className="absolute top-2 right-2 z-10 flex items-start gap-1"
        >
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              setMenuOpen((current) => !current);
            }}
            className={`rounded p-1 text-fg-faint transition-opacity hover:bg-active hover:text-fg ${
              menuOpen ? "opacity-100" : "opacity-0 group-hover/conversation:opacity-100"
            }`}
            title="Thread actions"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
              <circle cx="5" cy="12" r="1.8" />
              <circle cx="12" cy="12" r="1.8" />
              <circle cx="19" cy="12" r="1.8" />
            </svg>
          </button>
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              setMenuOpen(false);
              setConfirmAction("archive");
            }}
            className="rounded p-1 text-fg-faint opacity-0 transition-colors transition-opacity hover:bg-[#3a1418] hover:text-[#ffb4bb] group-hover/conversation:opacity-100"
            title="Archive thread"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="4" width="18" height="4" rx="1" />
              <path d="M5 8h14v10a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2Z" />
            </svg>
          </button>

          {menuOpen && (
            <div className="absolute top-full right-0 z-20 mt-2 min-w-40 rounded-xl border border-edge bg-[#232325] p-1 shadow-[0_14px_30px_rgba(0,0,0,0.35)]">
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  setMenuOpen(false);
                  setRenaming(true);
                }}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-fg transition-colors hover:bg-elevated"
                disabled={busyAction !== null}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 20h9" />
                  <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
                </svg>
                Rename
              </button>
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  setMenuOpen(false);
                  setConfirmAction("remove");
                }}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm text-[#ffb4bb] transition-colors hover:bg-[#3a1418] hover:text-[#ffd7dc]"
                disabled={busyAction !== null}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <line x1="18" y1="6" x2="6" y2="18" />
                  <line x1="6" y1="6" x2="18" y2="18" />
                </svg>
                Remove thread
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
