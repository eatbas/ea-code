import type { ReactNode } from "react";

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
    <aside className="flex h-full w-12 shrink-0 flex-col items-center gap-3 border-r border-[#313134] bg-[#151516] pt-8 pb-3">
      <button
        type="button"
        onClick={onToggle}
        className="rounded p-2 text-[#8b8b93] transition-colors hover:bg-[#202022] hover:text-[#f5f5f5]"
        title="Expand sidebar"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="9" y1="3" x2="9" y2="21" />
        </svg>
      </button>

      <div className="flex-1" />

      <button
        type="button"
        onClick={onSettingsClick}
        className={`rounded p-2 transition-colors ${
          settingsActive ? "bg-[#202022] text-[#f5f5f5]" : "text-[#8b8b93] hover:bg-[#202022] hover:text-[#f5f5f5]"
        }`}
        title="Settings"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 15 4.68a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06-.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </aside>
  );
}
