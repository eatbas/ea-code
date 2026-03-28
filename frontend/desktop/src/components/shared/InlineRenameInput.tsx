import type { ReactNode, RefObject } from "react";

interface InlineRenameInputProps {
  inputRef: RefObject<HTMLInputElement | null>;
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  placeholder?: string;
  disabled?: boolean;
}

/** Inline rename input with Cancel / Save buttons, shared by sidebar rows. */
export function InlineRenameInput({
  inputRef,
  value,
  onChange,
  onSubmit,
  onCancel,
  placeholder = "Name",
  disabled = false,
}: InlineRenameInputProps): ReactNode {
  return (
    <div className="rounded-lg border border-edge bg-input-bg px-3 py-3">
      <input
        ref={inputRef}
        type="text"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            onSubmit();
          }
          if (event.key === "Escape") {
            event.preventDefault();
            onCancel();
          }
        }}
        className="w-full rounded-md border border-edge bg-panel px-2 py-1.5 text-sm text-fg outline-none transition-colors focus:border-input-border-focus"
        placeholder={placeholder}
        disabled={disabled}
      />
      <div className="mt-2 flex items-center justify-end gap-2">
        <button
          type="button"
          onClick={onCancel}
          className="rounded px-2 py-1 text-[11px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
          disabled={disabled}
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={onSubmit}
          className="rounded bg-elevated px-2 py-1 text-[11px] font-medium text-fg transition-colors hover:bg-active"
          disabled={disabled}
        >
          Save
        </button>
      </div>
    </div>
  );
}
