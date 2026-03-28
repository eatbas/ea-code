import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ConversationSummary, ProjectEntry } from "../types";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarProjectList } from "./SidebarProjectList";
import { SidebarSettings } from "./SidebarSettings";

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
  conversationIndex: Record<string, ConversationSummary[]>;
  loadedProjectPaths: ReadonlySet<string>;
  loadingProjectPaths: ReadonlySet<string>;
  activeProjectPath?: string;
  activeConversationId?: string | null;
  onLoadProjectConversations: (projectPath: string) => Promise<void>;
  onSelectConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onCreateConversation: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  onReorderProjects: (orderedProjectPaths: string[]) => Promise<void>;
  onRemoveProject?: (projectPath: string) => void;
  onRenameProject?: (projectPath: string, name: string) => void;
  onArchiveProject?: (projectPath: string) => void;
  onUnarchiveProject?: (projectPath: string) => void;
  onRemoveConversation?: (projectPath: string, conversationId: string) => void;
  onRenameConversation?: (projectPath: string, conversationId: string, title: string) => void;
  onArchiveConversation?: (projectPath: string, conversationId: string) => void;
  onUnarchiveConversation?: (projectPath: string, conversationId: string) => void;
  onSetConversationPinned?: (projectPath: string, conversationId: string, pinned: boolean) => void;
}

export function Sidebar({
  collapsed,
  onToggle,
  activeView,
  onNavigate,
  projects,
  conversationIndex,
  loadedProjectPaths,
  loadingProjectPaths,
  activeProjectPath,
  activeConversationId,
  onLoadProjectConversations,
  onSelectConversation,
  onCreateConversation,
  onAddProject,
  onReorderProjects,
  onRemoveProject,
  onRenameProject,
  onArchiveProject,
  onUnarchiveProject,
  onRemoveConversation,
  onRenameConversation,
  onArchiveConversation,
  onUnarchiveConversation,
  onSetConversationPinned,
}: SidebarProps): ReactNode {
  const isSettings = activeView === "cli-setup";
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [showArchivedProjects, setShowArchivedProjects] = useState<boolean>(false);
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
  const visibleProjects = useMemo(
    () => showArchivedProjects ? projects : projects.filter((project) => !project.archivedAt),
    [projects, showArchivedProjects],
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
    <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-edge bg-panel">
      <div className="flex items-center justify-between px-3 pt-8 pb-3">
        <span className="text-sm font-medium text-fg">Projects</span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={onAddProject}
            className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
            title="Add project"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
          <button
            type="button"
            onClick={() => setShowArchivedProjects((current) => !current)}
            className={`rounded p-1.5 transition-colors ${
              showArchivedProjects
                ? "bg-elevated text-fg"
                : "text-fg-muted hover:bg-elevated hover:text-fg"
            }`}
            title={showArchivedProjects ? "Hide archived projects" : "Show archived projects"}
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </button>
          <button
            type="button"
            onClick={handleSettingsClick}
            className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
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
            className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
            title="Collapse sidebar"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
              <line x1="9" y1="3" x2="9" y2="21" />
            </svg>
          </button>
        </div>
      </div>

      <SidebarProjectList
        activeView={activeView}
        projects={projects}
        visibleProjects={visibleProjects}
        conversationIndex={conversationIndex}
        loadedProjectPaths={loadedProjectPaths}
        loadingProjectPaths={loadingProjectPaths}
        activeProjectPath={activeProjectPath}
        activeConversationId={activeConversationId}
        onLoadProjectConversations={onLoadProjectConversations}
        onSelectConversation={onSelectConversation}
        onCreateConversation={onCreateConversation}
        onReorderProjects={onReorderProjects}
        onRemoveProject={onRemoveProject}
        onRenameProject={onRenameProject}
        onArchiveProject={onArchiveProject}
        onUnarchiveProject={onUnarchiveProject}
        onRemoveConversation={onRemoveConversation}
        onRenameConversation={onRenameConversation}
        onArchiveConversation={onArchiveConversation}
        onUnarchiveConversation={onUnarchiveConversation}
        onSetConversationPinned={onSetConversationPinned}
      />

      <div className="border-t border-edge px-3 py-3">
        <p className="w-full text-center text-[10px] text-fg-faint" title={appFooterLabel}>
          {appFooterLabel}
        </p>
      </div>
    </aside>
  );
}
