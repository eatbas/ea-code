import { useEffect, useRef, useState } from "react";
import { useClickOutside } from "./useClickOutside";

export type ConfirmAction = "archive" | "remove";
export type BusyAction = "rename" | "archive" | "unarchive" | "remove" | "pin" | null;

interface UseSidebarRowMenuReturn {
  menuRef: React.RefObject<HTMLDivElement | null>;
  renameInputRef: React.RefObject<HTMLInputElement | null>;
  menuOpen: boolean;
  setMenuOpen: React.Dispatch<React.SetStateAction<boolean>>;
  renaming: boolean;
  setRenaming: React.Dispatch<React.SetStateAction<boolean>>;
  confirmAction: ConfirmAction | null;
  setConfirmAction: React.Dispatch<React.SetStateAction<ConfirmAction | null>>;
  renameValue: string;
  setRenameValue: React.Dispatch<React.SetStateAction<string>>;
  busyAction: BusyAction;
  setBusyAction: React.Dispatch<React.SetStateAction<BusyAction>>;
  /** Wraps an async action with busy-state tracking. */
  withBusy: <T>(action: BusyAction, fn: () => Promise<T>) => Promise<T | undefined>;
}

/** Shared state for sidebar row menus (rename, confirm, dropdown). */
export function useSidebarRowMenu(currentLabel: string): UseSidebarRowMenuReturn {
  const menuRef = useRef<HTMLDivElement | null>(null);
  const renameInputRef = useRef<HTMLInputElement | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [confirmAction, setConfirmAction] = useState<ConfirmAction | null>(null);
  const [renameValue, setRenameValue] = useState(currentLabel);
  const [busyAction, setBusyAction] = useState<BusyAction>(null);

  useClickOutside(menuRef, () => setMenuOpen(false), menuOpen);

  useEffect(() => {
    if (!renaming) {
      setRenameValue(currentLabel);
    }
  }, [currentLabel, renaming]);

  useEffect(() => {
    if (renaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renaming]);

  async function withBusy<T>(action: BusyAction, fn: () => Promise<T>): Promise<T | undefined> {
    setBusyAction(action);
    try {
      return await fn();
    } finally {
      setBusyAction(null);
    }
  }

  return {
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
    setBusyAction,
    withBusy,
  };
}
