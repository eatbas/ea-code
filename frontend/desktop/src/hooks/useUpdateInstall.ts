import { useRef, useState, useEffect } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import type { Update } from "@tauri-apps/plugin-updater";
import { useToast } from "../components/shared/Toast";

type UpdateStatus = "idle" | "queued" | "installing";

interface UseUpdateInstallReturn {
  status: UpdateStatus;
  updateVersion: string | null;
  /** Whether an install is currently running (stable ref-based check). */
  isInstalling: () => boolean;
  /** Version string of the last attempted install (avoids re-downloading). */
  getAttemptedVersion: () => string | null;
  /** Immediately install an update (skips the queue). */
  installUpdate: (update: Update) => void;
  /** Queue an update for later installation. */
  queueUpdate: (update: Update) => void;
  /** Install the queued update if conditions allow. */
  installPendingUpdate: () => void;
  /** Release resources held by a downloaded update. */
  closeUpdate: (update: Update | null) => void;
  /** Clean up on unmount — close any pending update. */
  dispose: () => void;
}

/**
 * Manages the download-and-install state machine for app updates.
 *
 * States: idle -> queued (update found while blocked) -> installing -> relaunch.
 */
export function useUpdateInstall(updatesBlocked: boolean): UseUpdateInstallReturn {
  const toast = useToast();
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);

  const installingRef = useRef(false);
  const attemptedVersionRef = useRef<string | null>(null);
  const pendingUpdateRef = useRef<Update | null>(null);
  const pendingRelaunchRef = useRef(false);
  const updatesBlockedRef = useRef(updatesBlocked);

  updatesBlockedRef.current = updatesBlocked;

  function setIdleState(): void {
    setStatus("idle");
    setUpdateVersion(null);
  }

  function closeUpdate(update: Update | null): void {
    if (!update) return;
    void update.close().catch(() => {
      // Ignore close failures; they do not affect update flow.
    });
  }

  function installUpdate(update: Update): void {
    if (installingRef.current) return;

    installingRef.current = true;
    attemptedVersionRef.current = update.version;
    pendingUpdateRef.current = null;
    setUpdateVersion(update.version);
    setStatus("installing");

    void update
      .downloadAndInstall()
      .then(() => {
        if (updatesBlockedRef.current) {
          pendingRelaunchRef.current = true;
          setStatus("queued");
          return;
        }
        void relaunch();
      })
      .catch(() => {
        installingRef.current = false;
        attemptedVersionRef.current = null;
        pendingRelaunchRef.current = false;
        setIdleState();
        closeUpdate(update);
        toast.error("Failed to install the update.");
      });
  }

  function queueUpdate(update: Update): void {
    const currentPending = pendingUpdateRef.current;
    if (currentPending?.version === update.version) {
      closeUpdate(update);
      return;
    }

    closeUpdate(currentPending);
    pendingUpdateRef.current = update;
    attemptedVersionRef.current = update.version;
    setUpdateVersion(update.version);
    setStatus("queued");
  }

  function installPendingUpdate(): void {
    const pendingUpdate = pendingUpdateRef.current;
    if (!pendingUpdate || updatesBlockedRef.current) return;
    installUpdate(pendingUpdate);
  }

  function dispose(): void {
    closeUpdate(pendingUpdateRef.current);
    pendingUpdateRef.current = null;
  }

  // When the blocker lifts, relaunch if the update already installed, or
  // install a queued update.
  useEffect(() => {
    if (!updatesBlocked) {
      if (pendingRelaunchRef.current) {
        void relaunch();
        return;
      }
      installPendingUpdate();
    }
  }, [updatesBlocked]);

  return {
    status,
    updateVersion,
    isInstalling: () => installingRef.current,
    getAttemptedVersion: () => attemptedVersionRef.current,
    installUpdate,
    queueUpdate,
    installPendingUpdate,
    closeUpdate,
    dispose,
  };
}
