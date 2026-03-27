import type { ReactNode } from "react";
import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";

type ToastVariant = "success" | "error" | "info";

interface ToastItem {
  id: number;
  message: string;
  variant: ToastVariant;
  action?: { label: string; onClick: () => void };
}

interface ToastContextValue {
  success: (message: string) => void;
  error: (message: string, action?: ToastItem["action"]) => void;
  info: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const VARIANT_CLASSES: Record<ToastVariant, string> = {
  success: "border-[#22c55e]/40 bg-[#22c55e]/10 text-[#86efac]",
  error: "border-[#ef4444]/40 bg-[#ef4444]/10 text-[#fca5a5]",
  info: "border-[#5a5a61]/60 bg-[#27272a] text-[#f5f5f5]",
};

const DISMISS_MS = 3500;

export function ToastProvider({ children }: { children: ReactNode }): ReactNode {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const nextIdRef = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const push = useCallback((message: string, variant: ToastVariant, action?: ToastItem["action"]) => {
    const id = nextIdRef.current++;
    setToasts((prev) => [...prev, { id, message, variant, action }]);
    setTimeout(() => dismiss(id), DISMISS_MS);
  }, [dismiss]);

  const success = useCallback((msg: string) => push(msg, "success"), [push]);
  const error = useCallback((msg: string, action?: ToastItem["action"]) => push(msg, "error", action), [push]);
  const info = useCallback((msg: string) => push(msg, "info"), [push]);

  const ctx = useMemo<ToastContextValue>(() => ({ success, error, info }), [success, error, info]);

  return (
    <ToastContext.Provider value={ctx}>
      {children}
      {toasts.length > 0 && (
        <div className="fixed right-6 top-6 z-[100] flex max-w-sm flex-col gap-2">
          {toasts.map((t) => (
            <div
              key={t.id}
              role="alert"
              className={`rounded-lg border px-4 py-3 text-sm shadow-lg ${VARIANT_CLASSES[t.variant]}`}
            >
              <div className="flex items-start justify-between gap-2">
                <p>{t.message}</p>
                <button
                  onClick={() => dismiss(t.id)}
                  className="shrink-0 opacity-60 hover:opacity-100 transition-opacity"
                  aria-label="Dismiss"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18" />
                    <line x1="6" y1="6" x2="18" y2="18" />
                  </svg>
                </button>
              </div>
              {t.action && (
                <button
                  onClick={() => {
                    t.action?.onClick();
                    dismiss(t.id);
                  }}
                  className="mt-2 text-xs font-medium underline opacity-80 hover:opacity-100 transition-opacity"
                >
                  {t.action.label}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </ToastContext.Provider>
  );
}

/** Global toast hook. Must be used inside <ToastProvider>. */
export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error("useToast must be used within a ToastProvider");
  return ctx;
}
