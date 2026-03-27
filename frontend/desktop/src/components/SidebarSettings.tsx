import type { ReactNode } from "react";
import type { ActiveView } from "../types";

interface SettingsNavItem {
  view: ActiveView;
  label: string;
  iconPath: string;
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
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
          Back to app
        </button>
      </div>

      <div className="flex flex-col gap-1 px-3">
        {navItems.map(({ view, label, iconPath }) => (
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
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              dangerouslySetInnerHTML={{ __html: iconPath }}
            />
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
