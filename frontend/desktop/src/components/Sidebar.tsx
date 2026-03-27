import type { ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import type { ActiveView, ConversationSummary, ProjectEntry } from "../types";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarSettings } from "./SidebarSettings";
import { folderName, formatRelativeTime } from "../utils/formatters";

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
  onRemoveConversation?: (projectPath: string, conversationId: string) => void;
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
  onRemoveConversation,
}: SidebarProps): ReactNode {
  const isSettings = activeView === "cli-setup";
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [projectPendingRemoval, setProjectPendingRemoval] = useState<string | null>(null);
  const [conversationPendingRemoval, setConversationPendingRemoval] = useState<string | null>(null);
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
    <aside className="flex h-full w-72 shrink-0 flex-col overflow-hidden border-r border-edge bg-panel">
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
          const projectHasRunningConversation = hasRunningConversation(conversations);
          return (
            <div key={project.path} className="group/project mb-2">
              <div className="relative">
                <button
                  type="button"
                  onClick={() => void onSelectProject(project.path)}
                  className={`flex w-full items-center gap-2 rounded-lg px-3 py-2 pr-16 text-left text-sm transition-colors ${
                    isActive
                      ? "bg-elevated text-fg"
                      : "text-fg-muted hover:bg-elevated hover:text-fg"
                  }`}
                  title={project.path}
                >
                  <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                  </svg>
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="truncate font-medium">{folderName(project.path)}</span>
                    {projectHasRunningConversation && (
                      <span
                        className="inline-flex h-2.5 w-2.5 shrink-0 rounded-full bg-[#1eb75f] shadow-[0_0_0_3px_rgba(30,183,95,0.16)] animate-pulse"
                        title="A conversation is running in this project"
                      />
                    )}
                  </span>
                </button>

                <div
                  className={`absolute top-1/2 right-2 flex -translate-y-1/2 items-center gap-1 transition-opacity ${
                    projectPendingRemoval === project.path ? "opacity-100" : "opacity-0 group-hover/project:opacity-100"
                  }`}
                >
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      setProjectPendingRemoval(null);
                      setConversationPendingRemoval(null);
                      void onCreateConversation(project.path);
                    }}
                    className="rounded p-1 text-fg-faint transition-colors hover:bg-active hover:text-fg"
                    title="New conversation"
                  >
                    <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                      <line x1="12" y1="5" x2="12" y2="19" />
                      <line x1="5" y1="12" x2="19" y2="12" />
                    </svg>
                  </button>
                  {onRemoveProject && (
                    projectPendingRemoval === project.path ? (
                      <>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            setProjectPendingRemoval(null);
                            setConversationPendingRemoval(null);
                          }}
                          className="rounded px-2 py-1 text-[10px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
                          title="Cancel project removal"
                        >
                          Cancel
                        </button>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            setProjectPendingRemoval(null);
                            setConversationPendingRemoval(null);
                            onRemoveProject(project.path);
                          }}
                          className="rounded bg-[#3a1418] px-2 py-1 text-[10px] font-medium text-[#ff8f98] transition-colors hover:bg-[#521a21] hover:text-[#ffd7dc]"
                          title={`Remove ${folderName(project.path)}`}
                        >
                          Delete
                        </button>
                      </>
                    ) : (
                      <button
                        type="button"
                        onClick={(event) => {
                          event.stopPropagation();
                          setProjectPendingRemoval(project.path);
                          setConversationPendingRemoval(null);
                        }}
                        className="rounded p-1 text-fg-faint transition-colors hover:bg-active hover:text-fg"
                        title="Remove project"
                      >
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                          <line x1="18" y1="6" x2="6" y2="18" />
                          <line x1="6" y1="6" x2="18" y2="18" />
                        </svg>
                      </button>
                    )
                  )}
                </div>
              </div>

              {conversations.length > 0 && (
                <div className="mt-1 space-y-1 pl-5">
                  {conversations.map((conversation) => {
                    const isConversationActive = isActive && conversation.id === activeConversationId;
                    return (
                      <div key={conversation.id} className="group/conversation relative">
                        <button
                          type="button"
                          onClick={() => {
                            void onSelectConversation(project.path, conversation.id);
                          }}
                          className={`flex w-full items-center justify-between gap-3 rounded-lg px-3 py-2 pr-8 text-left transition-colors ${
                            isConversationActive
                              ? "bg-[#252527] text-fg"
                              : "text-[#a3a3aa] hover:bg-[#1d1d1f] hover:text-fg"
                          }`}
                        >
                          <span className="flex min-w-0 items-center gap-2">
                            {conversation.status === "running" && (
                              <span
                                className="inline-flex h-2.5 w-2.5 shrink-0 rounded-full bg-[#1eb75f] shadow-[0_0_0_3px_rgba(30,183,95,0.16)] animate-pulse"
                                title="Conversation running"
                              />
                            )}
                            <span className="truncate text-sm">{conversation.title}</span>
                          </span>
                          <span className="shrink-0 text-xs text-fg-subtle">
                            {formatRelativeTime(conversation.updatedAt)}
                          </span>
                        </button>
                        {onRemoveConversation && (
                          conversationPendingRemoval === `${project.path}::${conversation.id}` ? (
                            <div className="absolute top-1/2 right-2 flex -translate-y-1/2 items-center gap-1 opacity-100">
                              <button
                                type="button"
                                onClick={(event) => {
                                  event.stopPropagation();
                                  setConversationPendingRemoval(null);
                                }}
                                className="rounded px-2 py-1 text-[10px] font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
                                title="Cancel conversation removal"
                              >
                                Cancel
                              </button>
                              <button
                                type="button"
                                onClick={(event) => {
                                  event.stopPropagation();
                                  setConversationPendingRemoval(null);
                                  onRemoveConversation(project.path, conversation.id);
                                }}
                                className="rounded bg-[#3a1418] px-2 py-1 text-[10px] font-medium text-[#ff8f98] transition-colors hover:bg-[#521a21] hover:text-[#ffd7dc]"
                                title={`Delete ${conversation.title}`}
                              >
                                Delete
                              </button>
                            </div>
                          ) : (
                            <button
                              type="button"
                              onClick={(event) => {
                                event.stopPropagation();
                                setConversationPendingRemoval(`${project.path}::${conversation.id}`);
                                setProjectPendingRemoval(null);
                              }}
                              className="absolute top-1/2 right-2 -translate-y-1/2 rounded p-1 text-fg-faint opacity-0 transition-opacity hover:bg-active hover:text-fg group-hover/conversation:opacity-100"
                              title="Delete conversation"
                            >
                              <svg xmlns="http://www.w3.org/2000/svg" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                                <line x1="18" y1="6" x2="6" y2="18" />
                                <line x1="6" y1="6" x2="18" y2="18" />
                              </svg>
                            </button>
                          )
                        )}
                      </div>
                    );
                  })}
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
