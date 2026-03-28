import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export interface RowDropdownMenuItem {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

interface RowDropdownMenuProps {
  items: RowDropdownMenuItem[];
}

/** Generic dropdown menu for sidebar row actions. */
export function RowDropdownMenu({ items }: RowDropdownMenuProps): ReactNode {
  return (
    <div className="absolute top-full right-0 z-20 mt-2 min-w-40 rounded-xl border border-edge bg-menu-surface p-1 shadow-[0_14px_30px_rgba(0,0,0,0.35)]">
      {items.map((item) => (
        <button
          key={item.label}
          type="button"
          onClick={(event) => {
            event.stopPropagation();
            item.onClick();
          }}
          className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
            item.danger
              ? "text-danger-text hover:bg-danger-bg hover:text-danger-text-hover"
              : "text-fg hover:bg-elevated"
          }`}
          disabled={item.disabled}
        >
          <item.icon size={14} strokeWidth={2} />
          {item.label}
        </button>
      ))}
    </div>
  );
}
