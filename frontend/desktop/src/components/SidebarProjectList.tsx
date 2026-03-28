import type { ReactNode } from "react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import type { ActiveView, ConversationSummary, ProjectEntry } from "../types";
import { projectDisplayName } from "../utils/formatters";
import { SidebarSortableProjectItem } from "./SidebarSortableProjectItem";

const EXPANDED_PROJECTS_STORAGE_KEY = "ea-code.sidebar.expanded-projects";

interface SidebarProjectListProps {
  activeView: ActiveView;
  projects: ProjectEntry[];
  visibleProjects: ProjectEntry[];
  conversationIndex: Record<string, ConversationSummary[]>;
  loadedProjectPaths: ReadonlySet<string>;
  loadingProjectPaths: ReadonlySet<string>;
  activeProjectPath?: string;
  activeConversationId?: string | null;
  onLoadProjectConversations: (projectPath: string) => Promise<void>;
  onSelectConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
  onCreateConversation: (projectPath: string) => void | Promise<void>;
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

function hasRunningConversation(conversations: ConversationSummary[]): boolean {
  return conversations.some((conversation) => conversation.status === "running");
}

function readExpandedProjects(): Set<string> {
  if (typeof window === "undefined") {
    return new Set();
  }

  try {
    const raw = window.localStorage.getItem(EXPANDED_PROJECTS_STORAGE_KEY);
    if (!raw) {
      return new Set();
    }

    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? new Set(parsed.filter((item): item is string => typeof item === "string")) : new Set();
  } catch {
    return new Set();
  }
}

function mergeProjectOrder(
  allProjects: ProjectEntry[],
  visibleProjects: ProjectEntry[],
  reorderedVisibleProjects: ProjectEntry[],
): string[] {
  const visibleProjectPaths = new Set(visibleProjects.map((project) => project.path));
  const reorderedPaths = reorderedVisibleProjects.map((project) => project.path);
  let reorderedIndex = 0;

  return allProjects.map((project) => {
    if (!visibleProjectPaths.has(project.path)) {
      return project.path;
    }

    const reorderedPath = reorderedPaths[reorderedIndex];
    reorderedIndex += 1;
    return reorderedPath;
  });
}

export function SidebarProjectList({
  activeView,
  projects,
  visibleProjects,
  conversationIndex,
  loadedProjectPaths,
  loadingProjectPaths,
  activeProjectPath,
  activeConversationId,
  onLoadProjectConversations,
  onSelectConversation,
  onCreateConversation,
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
}: SidebarProjectListProps): ReactNode {
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(() => readExpandedProjects());
  const [projectsShowingArchived, setProjectsShowingArchived] = useState<Set<string>>(new Set());
  const [draggingProjectPath, setDraggingProjectPath] = useState<string | null>(null);
  const expandedProjectsBeforeDragRef = useRef<Set<string> | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  useEffect(() => {
    setExpandedProjects((current) => {
      const validProjectPaths = new Set(projects.map((project) => project.path));
      const next = new Set([...current].filter((projectPath) => validProjectPaths.has(projectPath)));
      return next.size === current.size ? current : next;
    });
    setProjectsShowingArchived((current) => {
      const validProjectPaths = new Set(projects.map((project) => project.path));
      const next = new Set([...current].filter((projectPath) => validProjectPaths.has(projectPath)));
      return next.size === current.size ? current : next;
    });
  }, [projects]);

  useEffect(() => {
    if (typeof window === "undefined" || draggingProjectPath) {
      return;
    }

    window.localStorage.setItem(
      EXPANDED_PROJECTS_STORAGE_KEY,
      JSON.stringify([...expandedProjects]),
    );
  }, [draggingProjectPath, expandedProjects]);

  useEffect(() => {
    for (const projectPath of expandedProjects) {
      void onLoadProjectConversations(projectPath);
    }
  }, [expandedProjects, onLoadProjectConversations]);

  const draggingProject = useMemo(
    () => visibleProjects.find((project) => project.path === draggingProjectPath) ?? null,
    [draggingProjectPath, visibleProjects],
  );

  function restoreExpandedProjects(): void {
    const previousExpandedProjects = expandedProjectsBeforeDragRef.current;
    expandedProjectsBeforeDragRef.current = null;
    setDraggingProjectPath(null);
    if (previousExpandedProjects) {
      setExpandedProjects(new Set(previousExpandedProjects));
    }
  }

  function handleDragStart(event: DragStartEvent): void {
    const projectPath = String(event.active.id);
    expandedProjectsBeforeDragRef.current = new Set(expandedProjects);
    setDraggingProjectPath(projectPath);
    setExpandedProjects(new Set());
  }

  function handleDragEnd(event: DragEndEvent): void {
    const activeProjectPath = String(event.active.id);
    const overProjectPath = event.over ? String(event.over.id) : null;

    restoreExpandedProjects();

    if (!overProjectPath || activeProjectPath === overProjectPath) {
      return;
    }

    const oldIndex = visibleProjects.findIndex((project) => project.path === activeProjectPath);
    const newIndex = visibleProjects.findIndex((project) => project.path === overProjectPath);
    if (oldIndex < 0 || newIndex < 0) {
      return;
    }

    const reorderedVisibleProjects = arrayMove(visibleProjects, oldIndex, newIndex);
    void onReorderProjects(mergeProjectOrder(projects, visibleProjects, reorderedVisibleProjects));
  }

  if (activeView === "cli-setup") {
    return null;
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      onDragCancel={restoreExpandedProjects}
    >
      <SortableContext
        items={visibleProjects.map((project) => project.path)}
        strategy={verticalListSortingStrategy}
      >
        <div className="flex-1 overflow-y-auto px-2">
          {visibleProjects.length === 0 && (
            <p className="px-2 py-4 text-center text-xs text-fg-faint">
              {projects.length > 0 ? "Archived projects hidden." : "No projects yet. Add a project to get started."}
            </p>
          )}
          {visibleProjects.map((project) => {
            const isActive = project.path === activeProjectPath;
            const conversations = conversationIndex[project.path] ?? [];
            const showingArchivedConversations = projectsShowingArchived.has(project.path);
            const visibleConversations = showingArchivedConversations
              ? conversations
              : conversations.filter((conversation) => !conversation.archivedAt);

            return (
              <SidebarSortableProjectItem
                key={project.path}
                project={project}
                isActive={isActive}
                isExpanded={expandedProjects.has(project.path)}
                isDragging={draggingProjectPath === project.path}
                isLoaded={loadedProjectPaths.has(project.path)}
                isLoading={loadingProjectPaths.has(project.path)}
                hasRunningConversation={hasRunningConversation(conversations)}
                conversations={conversations}
                visibleConversations={visibleConversations}
                activeConversationId={activeConversationId}
                showingArchivedConversations={showingArchivedConversations}
                onProjectClick={() => {
                  if (!expandedProjects.has(project.path)) {
                    void onLoadProjectConversations(project.path);
                  }

                  setExpandedProjects((current) => {
                    const next = new Set(current);
                    if (next.has(project.path)) {
                      next.delete(project.path);
                    } else {
                      next.add(project.path);
                    }
                    return next;
                  });
                }}
                onCreateConversation={() => {
                  void onCreateConversation(project.path);
                }}
                onToggleShowArchivedConversations={() => {
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
                onSelectConversation={onSelectConversation}
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
            );
          })}
        </div>
      </SortableContext>

      <DragOverlay>
        {draggingProject && (
          <div className="mx-1 rounded-lg border border-edge-strong bg-drag-bg px-3 py-1.5 text-sm text-fg shadow-[0_18px_36px_rgba(0,0,0,0.35)]">
            {projectDisplayName(draggingProject)}
          </div>
        )}
      </DragOverlay>
    </DndContext>
  );
}
