import type { ReactNode } from "react";
import { PanelLeft, Settings } from "lucide-react";

interface SidebarCollapsedProps {
  onToggle: () => void;
  onSettingsClick: () => void;
  settingsActive: boolean;
}

export function SidebarCollapsed({
  onToggle,
  onSettingsClick,
  settingsActive,
}: SidebarCollapsedProps): ReactNode {
  return (
    <aside className="flex h-full w-12 shrink-0 flex-col items-center gap-3 border-r border-edge bg-panel pt-8 pb-3">
      <button
        type="button"
        onClick={onToggle}
        className="rounded p-2 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
        title="Expand sidebar"
      >
        <PanelLeft size={16} strokeWidth={2} />
      </button>

      <div className="flex-1" />

      <button
        type="button"
        onClick={onSettingsClick}
        className={`rounded p-2 transition-colors ${
          settingsActive ? "bg-elevated text-fg" : "text-fg-muted hover:bg-elevated hover:text-fg"
        }`}
        title="Settings"
      >
        <Settings size={16} strokeWidth={2} />
      </button>
    </aside>
  );
}
