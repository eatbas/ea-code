import type { ReactNode } from "react";
import { useState, useRef, useCallback } from "react";
import { Checkmark } from "./Checkmark";
import { useClickOutside } from "../../hooks/useClickOutside";

interface PopoverSelectOption {
  value: string;
  label: string;
}

interface PopoverSelectProps {
  value: string;
  options: PopoverSelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  /** "up" opens above the button, "down" opens below. */
  direction?: "up" | "down";
  /** Horizontal alignment of the popover relative to the button. */
  align?: "left" | "right";
  /** Controlled open state for coordinating sibling popovers. */
  open?: boolean;
  /** Called whenever the open state changes. */
  onOpenChange?: (open: boolean) => void;
  /** Called when the popover opens (useful for closing sibling popovers). */
  onOpen?: () => void;
  /** Optional trigger style override. */
  triggerClassName?: string;
  /** Optional menu style override. */
  menuClassName?: string;
}

/** Lightweight custom select rendered as a popover list. */
export function PopoverSelect({
  value,
  options,
  onChange,
  placeholder = "Select...",
  disabled = false,
  direction = "up",
  align = "left",
  open: controlledOpen,
  onOpenChange,
  onOpen,
  triggerClassName,
  menuClassName,
}: PopoverSelectProps): ReactNode {
  const [internalOpen, setInternalOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const open = controlledOpen ?? internalOpen;
  const setOpen = useCallback((next: boolean) => {
    if (controlledOpen === undefined) {
      setInternalOpen(next);
    }
    onOpenChange?.(next);
  }, [controlledOpen, onOpenChange]);
  const close = useCallback(() => setOpen(false), [setOpen]);
  useClickOutside(ref, close, open);

  const selectedLabel = options.find((o) => o.value === value)?.label ?? placeholder;

  const positionClasses = direction === "up"
    ? "bottom-full mb-2"
    : "top-full mt-2";
  const alignClasses = align === "right" ? "right-0" : "left-0";
  const resolvedTriggerClassName = triggerClassName
    ?? "flex h-9 items-center gap-2 rounded-full border border-[#34374d] bg-[#161925] px-3 text-xs font-medium text-[#f3f5fb] shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-[#4d5371] hover:bg-[#1b1f2d] disabled:cursor-not-allowed disabled:opacity-55";
  const resolvedMenuClassName = menuClassName
    ?? "w-max min-w-full rounded-2xl border border-[#34374d] bg-[#141722] p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur";

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        disabled={disabled || options.length === 0}
        onClick={() => {
          const next = !open;
          setOpen(next);
          if (next) onOpen?.();
        }}
        className={resolvedTriggerClassName}
      >
        <span className="min-w-0 flex-1 truncate text-left">{selectedLabel}</span>
        <svg className={`ml-auto h-3.5 w-3.5 shrink-0 text-[#98a0bf] transition-transform ${open ? "rotate-180" : ""}`} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.25">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
      {open && (
        <div className={`absolute ${positionClasses} ${alignClasses} z-50 ${resolvedMenuClassName}`}>
          {options.map((opt) => (
            <button
              type="button"
              key={opt.value}
              onClick={() => { onChange(opt.value); setOpen(false); }}
              className={`flex w-full items-center justify-between gap-3 rounded-xl px-3 py-2 text-left text-xs whitespace-nowrap transition-colors ${
                opt.value === value
                  ? "bg-[#212536] text-[#f5f7ff]"
                  : "text-[#a7aec9] hover:bg-[#1b1f2d] hover:text-[#f5f7ff]"
              }`}
            >
              <span>{opt.label}</span>
              {opt.value === value ? <Checkmark size="sm" className="text-[#8ce6a8]" /> : null}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
