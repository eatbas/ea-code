import type { ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ActiveView } from "../types";

export interface SettingsNavItem {
  view: ActiveView;
  label: string;
  icon: LucideIcon;
}

interface SidebarSettingsProps {
  activeView: ActiveView;
  onNavigate: (view: ActiveView) => void;
  onBackToApp: () => void;
  appFooterLabel: string;
  navItems: SettingsNavItem[];
}

export function SidebarSettings({
  activeView,
  onNavigate,
  onBackToApp,
  appFooterLabel,
  navItems,
}: SidebarSettingsProps): ReactNode {
  return (
    <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-edge bg-panel">
      <div className="px-3 pt-8 pb-3">
        <button
          type="button"
          onClick={onBackToApp}
          className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
        >
          <ArrowLeft size={14} strokeWidth={2} />
          Back to app
        </button>
      </div>

      <div className="flex flex-col gap-1 px-3">
        {navItems.map(({ view, label, icon: Icon }) => (
          <button
            key={view}
            type="button"
            onClick={() => onNavigate(view)}
            className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors ${
              activeView === view
                ? "bg-elevated text-fg"
                : "text-fg-muted hover:bg-elevated hover:text-fg"
            }`}
          >
            <Icon size={16} strokeWidth={2} />
            {label}
          </button>
        ))}
      </div>

      <div className="flex-1" />

      <div className="border-t border-edge px-3 py-3">
        <p className="w-full text-center text-[10px] text-fg-faint" title={appFooterLabel}>
          {appFooterLabel}
        </p>
      </div>
    </aside>
  );
}
