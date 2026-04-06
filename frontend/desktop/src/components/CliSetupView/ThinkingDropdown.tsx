import type { ReactNode } from "react";
import { PopoverSelect } from "../shared/PopoverSelect";

interface ThinkingOption {
  value: string;
  label: string;
}

interface ThinkingDropdownProps {
  /** Available thinking / effort options for this provider. */
  options: ThinkingOption[];
  /** Currently selected value (empty string = default / unset). */
  selected: string;
  /** Whether the control should be disabled. */
  disabled: boolean;
  /** Called when the user picks a new option. */
  onChange: (value: string) => void;
}

const TRIGGER_CLASS =
  "flex w-full h-10 items-center gap-2 rounded-md border border-edge-strong bg-input-bg px-3 text-sm font-medium text-fg shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55";

const MENU_CLASS =
  "w-full min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur";

/** Popover dropdown for selecting a provider's thinking / reasoning effort level. */
export function ThinkingDropdown({
  options,
  selected,
  disabled,
  onChange,
}: ThinkingDropdownProps): ReactNode {
  return (
    <div className="mt-4">
      <p className="mb-2 text-[10px] font-medium uppercase tracking-wider text-fg-faint">
        Thinking Level
      </p>
      <PopoverSelect
        value={selected}
        options={options}
        onChange={onChange}
        disabled={disabled}
        direction="down"
        placeholder="Default"
        triggerClassName={TRIGGER_CLASS}
        menuClassName={MENU_CLASS}
      />
    </div>
  );
}
