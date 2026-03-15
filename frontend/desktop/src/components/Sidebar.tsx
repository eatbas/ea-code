import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ProjectEntry, SessionMeta } from "../types";
import { ProjectThreadsList } from "./ProjectThreadsList";

// Re-export so existing consumers that import from Sidebar keep working.
export type { ActiveView } from "../types";

/** Data-driven settings navigation items. */
const SETTINGS_NAV_ITEMS: { view: ActiveView; label: string; iconPath: string }[] = [
  {
    view: "agents",
    label: "Agents",
    iconPath: '<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" /><path d="M23 21v-2a4 4 0 0 0-3-3.87" /><path d="M16 3.13a4 4 0 0 1 0 7.75" />',
  },
  {
    view: "cli-setup",
    label: "CLI Setup",
    iconPath: '<polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />',
  },
  {
    view: "mcp",
    label: "MCP Servers",
    iconPath: '<rect x="4" y="4" width="16" height="16" rx="2" /><path d="M9 9h6v6H9z" /><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3" />',
  },
  {
    view: "skills",
    label: "Skills",
    iconPath: '<path d="M12 3L3 7.5L12 12L21 7.5L12 3z" /><path d="M3 12l9 4.5l9-4.5" /><path d="M3 16.5L12 21l9-4.5" />',
  },
];


interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  onNewSession: () => void;
  activeView: ActiveView;
  onNavigate: (view: ActiveView) => void;
  projects: ProjectEntry[];
  activeProjectPath?: string;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  sessions: SessionMeta[];
  activeSessionId?: string;
  onSelectSession: (sessionId: string) => void;
  /** Session ID of the currently running pipeline (shows spinner on that session). */
  runningSessionId?: string;
  /** Archive (delete) a session by ID. */
  onArchiveSession?: (sessionId: string) => void;
}

/** Collapsible left sidebar with new thread and settings sub-navigation. */
export function Sidebar({
  collapsed,
  onToggle,
  onNewSession,
  activeView,
  onNavigate,
  projects,
  activeProjectPath,
  onSelectProject,
  onAddProject,
  sessions,
  activeSessionId,
  onSelectSession,
  runningSessionId,
  onArchiveSession,
}: SidebarProps): ReactNode {
  const isSettings = activeView === "agents" || activeView === "cli-setup" || activeView === "skills" || activeView === "mcp";
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const currentYear = new Date().getFullYear();

  useEffect(() => {
    let mounted = true;

    void getVersion()
      .then((version) => {
        if (mounted) {
          setAppVersion(version);
        }
      })
      .catch(() => {
        if (mounted) {
          setAppVersion(null);
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  const appFooterLabel = useMemo(
    () => `\u00A9 ${currentYear} ea-code${appVersion ? `\u00B7v${appVersion}` : ""}`,
    [appVersion, currentYear],
  );

  function handleSettingsClick(): void {
    onNavigate(isSettings ? "home" : "agents");
  }

  if (collapsed) {
    return (
      <aside className="flex h-full w-12 shrink-0 flex-col items-center border-r border-[#2e2e48] bg-[#1a1a24] pt-8 pb-3 gap-3">
        <button
          onClick={onToggle}
          className="rounded p-2 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="Expand sidebar"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <line x1="9" y1="3" x2="9" y2="21" />
          </svg>
        </button>

        <button
          onClick={onNewSession}
          className="rounded p-2 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="New thread"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
          </svg>
        </button>

        <div className="flex-1" />

        <button
          onClick={handleSettingsClick}
          className={`rounded p-2 transition-colors ${isSettings ? "bg-[#24243a] text-[#e4e4ed]" : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"}`}
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

  // Settings mode — full sidebar becomes settings navigation
  if (isSettings) {
    return (
      <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-[#2e2e48] bg-[#1a1a24]">
        {/* Back to app */}
        <div className="px-3 pt-8 pb-3">
          <button
            onClick={() => onNavigate("home")}
            className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="19" y1="12" x2="5" y2="12" />
              <polyline points="12 19 5 12 12 5" />
            </svg>
            Back to app
          </button>
        </div>

        {/* Settings nav items */}
        <div className="flex flex-col gap-1 px-3">
          {SETTINGS_NAV_ITEMS.map(({ view, label, iconPath }) => (
            <button
              key={view}
              onClick={() => onNavigate(view)}
              className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors ${
                activeView === view
                  ? "bg-[#24243a] text-[#e4e4ed]"
                  : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
              }`}
            >
              <svg
                xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
                dangerouslySetInnerHTML={{ __html: iconPath }}
              />
              {label}
            </button>
          ))}
        </div>

        <div className="flex-1" />

        <div className="border-t border-[#2e2e48] px-3 py-3">
          <p className="w-full text-center text-[10px] text-[#6b6b82]" title={appFooterLabel}>
            {appFooterLabel}
          </p>
        </div>
      </aside>
    );
  }

  // Default home sidebar
  return (
    <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-[#2e2e48] bg-[#1a1a24]">
      {/* Top bar — sidebar toggle + compose icon */}
      <div className="flex items-center px-3 pt-8 pb-3">
        <button
          onClick={onToggle}
          className="rounded p-1.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="Collapse sidebar"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
            <line x1="9" y1="3" x2="9" y2="21" />
          </svg>
        </button>
        <button
          onClick={onNewSession}
          className="ml-auto rounded p-1.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="New thread"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
          </svg>
        </button>
      </div>

      <ProjectThreadsList
        projects={projects}
        sessions={sessions}
        activeProjectPath={activeProjectPath}
        activeSessionId={activeSessionId}
        runningSessionId={runningSessionId}
        onSelectProject={onSelectProject}
        onSelectSession={onSelectSession}
        onArchiveSession={onArchiveSession}
      />

      <div className="px-3 pb-3">
        <button
          onClick={onAddProject}
          className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v2" />
            <path d="M3 11v6a2 2 0 0 0 2 2h6" />
            <line x1="16" y1="17" x2="22" y2="17" />
            <line x1="19" y1="14" x2="19" y2="20" />
          </svg>
          Add project
        </button>
      </div>

      {/* Bottom — settings */}
      <div className="border-t border-[#2e2e48] p-3">
        <button
          onClick={handleSettingsClick}
          className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06-.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 15 4.68a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06-.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09A1.65 1.65 0 0 0 19.4 15a1.65 1.65 0 0 0 1.51 1z" />
          </svg>
          Settings
        </button>
        <p className="mt-2 w-full text-center text-[10px] text-[#6b6b82]" title={appFooterLabel}>
          {appFooterLabel}
        </p>
      </div>
    </aside>
  );
}
