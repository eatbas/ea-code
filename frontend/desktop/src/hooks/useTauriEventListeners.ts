import { useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { disposeTauriListener } from "../utils/tauriListeners";

/** Descriptor for a single Tauri event listener to register. */
export interface EventListenerConfig<T = unknown> {
  /** The Tauri event name to listen for. */
  event: string;
  /** Handler invoked with the event payload each time the event fires. */
  handler: (payload: T) => void;
}

interface UseTauriEventListenersOptions {
  /**
   * One or more event listeners to register on mount.
   * Handlers should call external setters (e.g. from useState) to update state.
   *
   * Uses `any` deliberately: each caller types its own handler generics, and
   * the array erases those types at the collection boundary. Runtime safety is
   * guaranteed by Tauri's typed `listen<T>` at each call site.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  listeners: EventListenerConfig<any>[];
  /**
   * Optional "done" event that, when received, sets `checking` to `false`.
   * If omitted the `checking` state is never managed by this hook.
   */
  doneEvent?: string;
}

interface UseTauriEventListenersReturn {
  /** Whether the operation is still in progress (resets to false on doneEvent). */
  checking: boolean;
  /** Set to `true` when kicking off a new check; the done event resets it. */
  setChecking: (value: boolean) => void;
}

/**
 * Registers one or more Tauri event listeners on mount and tears them down on
 * unmount. Optionally manages a `checking` boolean that resets when a
 * designated "done" event fires.
 *
 * This hook does NOT own any domain state — callers pass their own state
 * setters inside each listener's `handler`. It exists purely to deduplicate
 * the listen/unlisten boilerplate shared across event-driven hooks.
 */
export function useTauriEventListeners(
  options: UseTauriEventListenersOptions,
): UseTauriEventListenersReturn {
  const [checking, setChecking] = useState<boolean>(false);

  // Keep a stable reference to options so the effect only runs once.
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const { listeners, doneEvent } = optionsRef.current;
    const unlistenPromises: Promise<UnlistenFn>[] = [];

    for (const { event, handler } of listeners) {
      unlistenPromises.push(
        listen(event, (e) => {
          handler(e.payload);
        }),
      );
    }

    if (doneEvent) {
      unlistenPromises.push(
        listen<void>(doneEvent, () => {
          setChecking(false);
        }),
      );
    }

    return () => {
      for (const promise of unlistenPromises) {
        disposeTauriListener(promise, "event");
      }
    };
  }, []); // Intentionally mount-only; handlers read live state via closures / setters.

  return { checking, setChecking };
}
