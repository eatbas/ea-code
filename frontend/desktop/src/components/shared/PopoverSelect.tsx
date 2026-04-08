import type { ReactNode } from "react";
import { useState, useRef, useCallback } from "react";
import { ChevronDown } from "lucide-react";
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
  /** Map from option value to a shorter label shown on the trigger button.
   *  When provided, the menu still displays the full `label` from `options`. */
  triggerLabels?: Record<string, string>;
  /** Optional title shown at the top of the dropdown menu. */
  menuTitle?: string;
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
  triggerLabels,
  menuTitle,
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

  const selectedOption = options.find((o) => o.value === value);
  const selectedLabel = (triggerLabels && selectedOption ? triggerLabels[selectedOption.value] : undefined)
    ?? selectedOption?.label
    ?? placeholder;

  const positionClasses = direction === "up"
    ? "bottom-full mb-2"
    : "top-full mt-2";
  const alignClasses = align === "right" ? "right-0" : "left-0";
  const resolvedTriggerClassName = triggerClassName
    ?? "flex h-9 items-center gap-2 rounded-lg border border-edge-strong bg-input-bg px-3 text-xs font-medium text-fg shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55";
  const resolvedMenuClassName = menuClassName
    ?? "w-max min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur";

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
        <ChevronDown
          size={14}
          className={`ml-auto shrink-0 text-fg-muted transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>
      {open && (
        <div className={`absolute ${positionClasses} ${alignClasses} z-50 ${resolvedMenuClassName}`}>
          {menuTitle && (
            <p className="px-3 pb-1 pt-1.5 text-[10px] font-semibold uppercase tracking-wider text-fg-faint">
              {menuTitle}
            </p>
          )}
          {options.map((opt) => (
            <button
              type="button"
              key={opt.value}
              onClick={() => { onChange(opt.value); setOpen(false); }}
              className={`flex w-full items-center justify-between gap-3 rounded-xl px-3 py-2 text-left text-xs whitespace-nowrap transition-colors ${
                opt.value === value
                  ? "bg-elevated text-fg"
                  : "text-option-muted hover:bg-input-bg hover:text-fg"
              }`}
            >
              <span>{opt.label}</span>
              {opt.value === value ? <Checkmark size="sm" className="text-success-chip-text" /> : null}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
