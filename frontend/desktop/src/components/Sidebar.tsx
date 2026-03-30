import type { ReactNode } from "react";
import { useMemo, useState } from "react";
import { Cpu, Eye, PanelLeft, Plus, Settings, TerminalSquare } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ActiveView, ConversationSummary, ProjectEntry } from "../types";
import { useAppVersion } from "../hooks/useAppVersion";
import { SidebarCollapsed } from "./SidebarCollapsed";
import { SidebarProjectList } from "./SidebarProjectList";
import { SidebarSettings } from "./SidebarSettings";

const SETTINGS_NAV_ITEMS: { view: ActiveView; label: string; icon: LucideIcon }[] = [
  { view: "agents", label: "Agents", icon: Cpu },
  { view: "cli-setup", label: "CLI Setup", icon: TerminalSquare },
];

const SETTINGS_VIEWS: ReadonlySet<ActiveView> = new Set(
  SETTINGS_NAV_ITEMS.map((item) => item.view),
);

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
  const isSettings = SETTINGS_VIEWS.has(activeView);
  const appVersion = useAppVersion();
  const [showArchivedProjects, setShowArchivedProjects] = useState<boolean>(false);
  const currentYear = new Date().getFullYear();

  const appFooterLabel = useMemo(
    () => `\u00A9 ${currentYear} maestro${appVersion ? `\u00B7v${appVersion}` : ""}`,
    [appVersion, currentYear],
  );
  const visibleProjects = useMemo(
    () => showArchivedProjects ? projects : projects.filter((project) => !project.archivedAt),
    [projects, showArchivedProjects],
  );

  function handleSettingsClick(): void {
    onNavigate(isSettings ? "home" : SETTINGS_NAV_ITEMS[0].view);
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
            <Plus size={14} strokeWidth={2} />
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
            <Eye size={14} strokeWidth={2} />
          </button>
          <button
            type="button"
            onClick={handleSettingsClick}
            className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
            title="Settings"
          >
            <Settings size={14} strokeWidth={2} />
          </button>
          <button
            type="button"
            onClick={onToggle}
            className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
            title="Collapse sidebar"
          >
            <PanelLeft size={14} strokeWidth={2} />
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
