import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ProjectEntry } from "../types";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarSettings } from "./SidebarSettings";
import { folderName } from "../utils/formatters";

// Re-export so existing consumers that import from Sidebar keep working.
export type { ActiveView } from "../types";

/** Data-driven settings navigation items. */
const SETTINGS_NAV_ITEMS: { view: ActiveView; label: string; iconPath: string }[] = [
  {
    view: "cli-setup",
    label: "CLI Setup",
    iconPath: '<polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />',
  },
];

interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  activeView: ActiveView;
  onNavigate: (view: ActiveView) => void;
  projects: ProjectEntry[];
  activeProjectPath?: string;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  onRemoveProject?: (projectPath: string) => void;
}

/** Collapsible left sidebar with project list and settings sub-navigation. */
export function Sidebar({
  collapsed,
  onToggle,
  activeView,
  onNavigate,
  projects,
  activeProjectPath,
  onSelectProject,
  onAddProject,
  onRemoveProject,
}: SidebarProps): ReactNode {
  const isSettings = activeView === "cli-setup";
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
    onNavigate(isSettings ? "home" : "cli-setup");
  }

  if (collapsed) {
    return (
      <SidebarCollapsed
        onToggle={onToggle}
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
    <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-[#2e2e48] bg-[#1a1a24]">
      {/* Header */}
      <div className="flex items-center justify-between px-3 pt-8 pb-3">
        <span className="text-sm font-medium text-[#e4e4ed]">Projects</span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={onAddProject}
            className="rounded p-1.5 text-[#9898b0] transition-colors hover:bg-[#24243a] hover:text-[#e4e4ed]"
            title="Add project"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
          <button
            type="button"
            onClick={handleSettingsClick}
            className="rounded p-1.5 text-[#9898b0] transition-colors hover:bg-[#24243a] hover:text-[#e4e4ed]"
            title="Settings"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3" />
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09A1.65 1.65 0 0 0 15 4.68a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06-.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
            </svg>
          </button>
          <button
            type="button"
            onClick={onToggle}
            className="rounded p-1.5 text-[#9898b0] transition-colors hover:bg-[#24243a] hover:text-[#e4e4ed]"
            title="Collapse sidebar"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <line x1="9" y1="3" x2="9" y2="21" />
            </svg>
          </button>
        </div>
      </div>

      {/* Project list */}
      <div className="flex-1 overflow-y-auto px-2">
        {projects.length === 0 && (
          <p className="px-2 py-4 text-center text-xs text-[#6b6b80]">No projects yet. Add a project to get started.</p>
        )}
        {projects.map((project) => {
          const isActive = project.path === activeProjectPath;
          return (
            <div key={project.path} className="group relative">
              <button
                type="button"
                onClick={() => void onSelectProject(project.path)}
                className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 text-left text-sm transition-colors ${
                  isActive
                    ? "bg-[#24243a] text-[#e4e4ed]"
                    : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                }`}
                title={project.path}
              >
                <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                </svg>
                <span className="truncate">{folderName(project.path)}</span>
              </button>
              {onRemoveProject && (
                <button
                  type="button"
                  onClick={() => onRemoveProject(project.path)}
                  className="absolute top-1/2 right-2 -translate-y-1/2 rounded p-1 text-[#6b6b80] opacity-0 transition-opacity hover:text-[#e4e4ed] group-hover:opacity-100"
                  title="Remove project"
                >
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <line x1="18" y1="6" x2="6" y2="18" />
                    <line x1="6" y1="6" x2="18" y2="18" />
                  </svg>
                </button>
              )}
            </div>
          );
        })}
      </div>

      {/* Footer */}
      <div className="border-t border-[#2e2e48] px-3 py-3">
        <p className="w-full text-center text-[10px] text-[#6b6b82]" title={appFooterLabel}>
          {appFooterLabel}
        </p>
      </div>
    </aside>
  );
}
