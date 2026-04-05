import type { UnlistenFn } from "@tauri-apps/api/event";

/**
 * Dispose a Tauri listener without surfacing teardown races as UI crashes.
 *
 * Listener registration resolves asynchronously, and by the time a component
 * unmounts the underlying listener may already be gone during reloads, route
 * changes, or app shutdown. Those cases are safe to ignore.
 */
export function disposeTauriListener(
  unlistenPromise: Promise<UnlistenFn>,
  label: string,
): void {
  void unlistenPromise
    .then((unlisten) => Promise.resolve(unlisten()).catch((error: unknown) => {
      console.warn(`[tauri-listener] Failed to unregister ${label}:`, error);
    }))
    .catch((error: unknown) => {
      console.warn(`[tauri-listener] Failed to resolve ${label} listener:`, error);
    });
}
