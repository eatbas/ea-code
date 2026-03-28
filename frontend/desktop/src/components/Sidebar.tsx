import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ConversationSummary, ProjectEntry } from "../types";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarConversationRow } from "./SidebarConversationRow";
import { SidebarProjectRow } from "./SidebarProjectRow";
import { SidebarSettings } from "./SidebarSettings";
import { projectDisplayName } from "../utils/formatters";

/** Data-driven settings navigation items. */
const SETTINGS_NAV_ITEMS: { view: ActiveView; label: string; iconPath: string }[] = [
  {
    view: "cli-setup",
    label: "CLI Setup",
    iconPath: '<polyline points="4 17 10 11 4 5" /><line x1="12" y1="19" x2="20" y2="19" />',
  },
];

function hasRunningConversation(conversations: ConversationSummary[]): boolean {
  return conversations.some((conversation) => conversation.status === "running");
}

interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  activeView: ActiveView;
  onNavigate: (view: ActiveView) => void;
  projects: ProjectEntry[];
  conversationIndex: Record<string, ConversationSummary[]>;
  activeProjectPath?: string;
  activeConversationId?: string | null;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onSelectConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onCreateConversation: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  onRemoveProject?: (projectPath: string) => void;
  onRenameProject?: (projectPath: string, name: string) => void;
  onArchiveProject?: (projectPath: string) => void;
  onRemoveConversation?: (projectPath: string, conversationId: string) => void;
  onRenameConversation?: (projectPath: string, conversationId: string, title: string) => void;
  onArchiveConversation?: (projectPath: string, conversationId: string) => void;
  onUnarchiveConversation?: (projectPath: string, conversationId: string) => void;
  onSetConversationPinned?: (projectPath: string, conversationId: string, pinned: boolean) => void;
}

/** Collapsible left sidebar with project list and settings sub-navigation. */
export function Sidebar({
  collapsed,
  onToggle,
  activeView,
  onNavigate,
  projects,
  conversationIndex,
  activeProjectPath,
  activeConversationId,
  onSelectProject,
  onSelectConversation,
  onCreateConversation,
  onAddProject,
  onRemoveProject,
  onRenameProject,
  onArchiveProject,
  onRemoveConversation,
  onRenameConversation,
  onArchiveConversation,
  onUnarchiveConversation,
  onSetConversationPinned,
}: SidebarProps): ReactNode {
  const isSettings = activeView === "cli-setup";
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [projectsShowingArchived, setProjectsShowingArchived] = useState<Set<string>>(new Set());
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

  useEffect(() => {
    setExpandedProjects((current) => {
      const next = new Set(current);
      let changed = false;

      for (const projectPath of next) {
        if (!projects.some((project) => project.path === projectPath)) {
          next.delete(projectPath);
          changed = true;
        }
      }

      if (activeProjectPath && !next.has(activeProjectPath)) {
        next.add(activeProjectPath);
        changed = true;
      }

      return changed ? next : current;
    });
  }, [activeProjectPath, projects]);

  useEffect(() => {
    setProjectsShowingArchived((current) => {
      const validProjectPaths = new Set(projects.map((project) => project.path));
      const next = new Set(
        [...current].filter((projectPath) => validProjectPaths.has(projectPath)),
      );
      return next.size === current.size ? current : next;
    });
  }, [projects]);

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
    <aside className="flex h-full w-60 shrink-0 flex-col overflow-hidden border-r border-edge bg-panel">
      {/* Header */}
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

      {/* Project list */}
      <div className="flex-1 overflow-y-auto px-2">
        {projects.length === 0 && (
          <p className="px-2 py-4 text-center text-xs text-fg-faint">No projects yet. Add a project to get started.</p>
        )}
        {projects.map((project) => {
          const isActive = project.path === activeProjectPath;
          const conversations = conversationIndex[project.path] ?? [];
          const showingArchived = projectsShowingArchived.has(project.path);
          const visibleConversations = showingArchived
            ? conversations
            : conversations.filter((conversation) => !conversation.archivedAt);
          const projectHasRunningConversation = hasRunningConversation(visibleConversations);
          const projectExpanded = expandedProjects.has(project.path);
          return (
            <div key={project.path} className="mb-2">
              <SidebarProjectRow
                projectPath={project.path}
                projectLabel={projectDisplayName(project)}
                isActive={isActive}
                expanded={projectExpanded}
                hasConversations={conversations.length > 0}
                hasRunningConversation={projectHasRunningConversation}
                showingArchived={showingArchived}
                onProjectClick={() => {
                  if (isActive) {
                    setExpandedProjects((current) => {
                      const next = new Set(current);
                      if (next.has(project.path)) {
                        next.delete(project.path);
                      } else {
                        next.add(project.path);
                      }
                      return next;
                    });
                    return;
                  }

                  setExpandedProjects((current) => {
                    if (current.has(project.path)) {
                      return current;
                    }

                    const next = new Set(current);
                    next.add(project.path);
                    return next;
                  });
                  void onSelectProject(project.path);
                }}
                onCreateConversation={() => {
                  void onCreateConversation(project.path);
                }}
                onToggleShowArchived={() => {
                  setProjectsShowingArchived((current) => {
                    const next = new Set(current);
                    if (next.has(project.path)) {
                      next.delete(project.path);
                    } else {
                      next.add(project.path);
                    }
                    return next;
                  });
                }}
                onRemoveProject={onRemoveProject
                  ? () => {
                      onRemoveProject(project.path);
                    }
                  : undefined}
                onRenameProject={onRenameProject
                  ? (name) => {
                      onRenameProject(project.path, name);
                    }
                  : undefined}
                onArchiveProject={onArchiveProject
                  ? () => {
                      onArchiveProject(project.path);
                    }
                  : undefined}
              />

              {conversations.length > 0 && projectExpanded && (
                <div className="mt-1 space-y-1">
                  {visibleConversations.length === 0 && (
                    <p className="px-3 py-2 text-xs text-fg-faint">Archived conversations hidden</p>
                  )}
                  {visibleConversations.map((conversation) => (
                    <SidebarConversationRow
                      key={conversation.id}
                      conversation={conversation}
                      isActive={isActive && conversation.id === activeConversationId}
                      projectPath={project.path}
                      onSelectConversation={onSelectConversation}
                      onRenameConversation={onRenameConversation ?? (() => undefined)}
                      onArchiveConversation={onArchiveConversation ?? (() => undefined)}
                      onUnarchiveConversation={onUnarchiveConversation ?? (() => undefined)}
                      onRemoveConversation={onRemoveConversation ?? (() => undefined)}
                      onSetConversationPinned={onSetConversationPinned ?? (() => undefined)}
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Footer */}
      <div className="border-t border-edge px-3 py-3">
        <p className="w-full text-center text-[10px] text-fg-faint" title={appFooterLabel}>
          {appFooterLabel}
        </p>
      </div>
    </aside>
  );
}
