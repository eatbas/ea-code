import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ProjectEntry, SessionMeta } from "../types";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarHome } from "./SidebarHome";
import { SidebarSettings } from "./SidebarSettings";

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
      <SidebarCollapsed
        onToggle={onToggle}
        onNewSession={onNewSession}
        onSettingsClick={handleSettingsClick}
        settingsActive={isSettings}
      />
    );
  }

  if (isSettings) {
    return (
      <SidebarSettings
        activeView={activeView}
        onNavigate={onNavigate}
        onBackToApp={() => onNavigate("home")}
        appFooterLabel={appFooterLabel}
        navItems={SETTINGS_NAV_ITEMS}
      />
    );
  }

  return (
    <SidebarHome
      onToggle={onToggle}
      onNewSession={onNewSession}
      onSettingsClick={handleSettingsClick}
      appFooterLabel={appFooterLabel}
      projects={projects}
      activeProjectPath={activeProjectPath}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
      sessions={sessions}
      activeSessionId={activeSessionId}
      onSelectSession={onSelectSession}
      runningSessionId={runningSessionId}
      onArchiveSession={onArchiveSession}
    />
  );
}
