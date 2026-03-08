import type { ReactNode } from "react";
import { useState, useRef, useEffect, useCallback } from "react";

interface PopoverSelectOption {
  value: string;
  label: string;
}

interface PopoverSelectProps {
  value: string;
  options: PopoverSelectOption[];
  onChange: (value: string) => void;
  /** "up" opens above the button, "down" opens below. */
  direction?: "up" | "down";
  /** Horizontal alignment of the popover relative to the button. */
  align?: "left" | "right";
  /** Called when the popover opens (useful for closing sibling popovers). */
  onOpen?: () => void;
}

/** Lightweight custom select rendered as a popover list. */
export function PopoverSelect({
  value,
  options,
  onChange,
  direction = "up",
  align = "left",
  onOpen,
}: PopoverSelectProps): ReactNode {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent): void {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  // Close on Escape
  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === "Escape") setOpen(false);
  }, []);
  useEffect(() => {
    if (!open) return;
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [open, handleEscape]);

  const selectedLabel = options.find((o) => o.value === value)?.label ?? value;

  const positionClasses = direction === "up"
    ? "bottom-full mb-1"
    : "top-full mt-1";
  const alignClasses = align === "right" ? "right-0" : "left-0";

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => {
          const next = !open;
          setOpen(next);
          if (next) onOpen?.();
        }}
        className="flex items-center gap-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-2 py-1 text-xs text-[#e4e4ed] hover:border-[#6366f1] transition-colors"
      >
        <span>{selectedLabel}</span>
        <svg className="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
      {open && (
        <div className={`absolute ${positionClasses} ${alignClasses} z-50 min-w-full rounded-lg border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg`}>
          {options.map((opt) => (
            <button
              key={opt.value}
              onClick={() => { onChange(opt.value); setOpen(false); }}
              className={`flex w-full items-center px-3 py-1.5 text-xs whitespace-nowrap transition-colors ${
                opt.value === value
                  ? "bg-[#24243a] text-[#e4e4ed]"
                  : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
              }`}
            >{opt.label}</button>
          ))}
        </div>
      )}
    </div>
  );
}
