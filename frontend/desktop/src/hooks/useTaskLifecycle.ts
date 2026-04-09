import { useEffect, useRef } from "react";
import type { AppSettings, ConversationStatusEvent } from "../types";
import { CONVERSATION_EVENTS } from "../constants/events";
import { useTauriEventListeners } from "./useTauriEventListeners";
import { enableKeepAwake, disableKeepAwake, sendNotification } from "../lib/desktopApi";

/** Statuses that mean the current run is done and can notify the user. */
const NOTIFIABLE_FINAL: ReadonlySet<string> = new Set(["completed", "failed", "stopped", "awaiting_review"]);

/**
 * Global hook that reacts to conversation status transitions to:
 * 1. Auto-manage keep-awake during active tasks or when manually enabled.
 * 2. Send OS-level notifications when tasks finish (respecting user settings).
 *
 * Mount this once at the App root.
 */
export function useTaskLifecycle(settings: AppSettings | null): void {
  const runningIds = useRef<Set<string>>(new Set());
  const keepAwakeForTasks = useRef<boolean>(false);
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  /** Enable keep-awake whilst tasks are running, or for the full session when manually enabled. */
  function syncKeepAwake(): void {
    const shouldBeAwake = runningIds.current.size > 0 || (settingsRef.current?.keepAwake ?? false);

    if (shouldBeAwake && !keepAwakeForTasks.current) {
      keepAwakeForTasks.current = true;
      enableKeepAwake().catch(() => { /* best-effort */ });
    } else if (!shouldBeAwake && keepAwakeForTasks.current) {
      keepAwakeForTasks.current = false;
      disableKeepAwake().catch(() => { /* best-effort */ });
    }
  }

  /** Send an OS notification based on the user's completion notification setting. */
  function notifyCompletion(title: string, status: string): void {
    const pref = settingsRef.current?.completionNotifications ?? "never";
    if (pref === "never") return;
    if (pref === "when_in_background" && !document.hidden) return;

    const body = status === "completed"
      ? "Task completed successfully."
      : status === "awaiting_review"
        ? "Task is ready for review."
      : status === "failed"
        ? "Task failed."
        : "Task stopped.";

    sendNotification(title, body).catch((error: unknown) => {
      console.error("Failed to send notification.", error);
    });
  }

  function handleStatus(event: ConversationStatusEvent): void {
    const { id, title, status } = event.conversation;

    if (status === "running") {
      runningIds.current.add(id);
      syncKeepAwake();
    } else if (NOTIFIABLE_FINAL.has(status)) {
      const wasRunning = runningIds.current.delete(id);
      syncKeepAwake();

      if (wasRunning) {
        notifyCompletion(title || "Maestro", status);
      }
    }
  }

  useTauriEventListeners({
    listeners: [
      { event: CONVERSATION_EVENTS.STATUS, handler: handleStatus },
    ],
  });

  // Sync keep-awake when the manual toggle changes or settings finish loading.
  useEffect(() => {
    syncKeepAwake();
  }, [settings?.keepAwake]);

  // Clean up on unmount.
  useEffect(() => {
    return () => {
      if (keepAwakeForTasks.current) {
        disableKeepAwake().catch(() => { /* best-effort */ });
      }
    };
  }, []);
}
