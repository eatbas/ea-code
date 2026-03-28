import type { ReactNode } from "react";

interface ConfirmActionPopoverProps {
  label: string;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
  disabled?: boolean;
}

/** Inline confirm/cancel popover for destructive actions. */
export function ConfirmActionPopover({
  label,
  confirmLabel,
  onConfirm,
  onCancel,
  disabled = false,
}: ConfirmActionPopoverProps): ReactNode {
  return (
    <div className="flex items-center gap-1 rounded-lg border border-edge bg-menu-surface px-1.5 py-1 shadow-[0_10px_24px_rgba(0,0,0,0.28)]">
      <span className="px-1 text-[11px] text-fg-muted">{label}</span>
      <button
        type="button"
        onClick={(event) => {
          event.stopPropagation();
          onCancel();
        }}
        className="rounded px-2 py-1 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
        disabled={disabled}
      >
        Cancel
      </button>
      <button
        type="button"
        onClick={(event) => {
          event.stopPropagation();
          onConfirm();
        }}
        className="rounded bg-danger-bg px-2 py-1 text-[11px] font-medium text-danger-text transition-colors hover:bg-danger-bg-hover hover:text-danger-text-hover"
        disabled={disabled}
      >
        {confirmLabel}
      </button>
    </div>
  );
}
