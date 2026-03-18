import type { ReactNode } from "react";

interface SidebarCollapsedProps {
  onToggle: () => void;
  onNewSession: () => void;
  onSettingsClick: () => void;
  settingsActive: boolean;
}

export function SidebarCollapsed({
  onToggle,
  onNewSession,
  onSettingsClick,
  settingsActive,
}: SidebarCollapsedProps): ReactNode {
  return (
    <aside className="flex h-full w-12 shrink-0 flex-col items-center gap-3 border-r border-[#2e2e48] bg-[#1a1a24] pt-8 pb-3">
      <button
        type="button"
        onClick={onToggle}
        className="rounded p-2 text-[#9898b0] transition-colors hover:bg-[#24243a] hover:text-[#e4e4ed]"
        title="Expand sidebar"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
          <line x1="9" y1="3" x2="9" y2="21" />
        </svg>
      </button>

      <button
        type="button"
        onClick={onNewSession}
        className="rounded p-2 text-[#9898b0] transition-colors hover:bg-[#24243a] hover:text-[#e4e4ed]"
        title="New thread"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
          <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
      </button>

      <div className="flex-1" />

      <button
        type="button"
        onClick={onSettingsClick}
        className={`rounded p-2 transition-colors ${
          settingsActive ? "bg-[#24243a] text-[#e4e4ed]" : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
        }`}
        title="Settings"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 15 4.68a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06-.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09A1.65 1.65 0 0 0 19.4 15a1.65 1.65 0 0 0 1.51 1z" />
        </svg>
      </button>
    </aside>
  );
}
