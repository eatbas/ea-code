import type { CSSProperties, ReactNode } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { ConversationSummary, ProjectEntry } from "../../types";
import { projectDisplayName } from "../../utils/formatters";
import { ConversationRow } from "./ConversationRow";
import { ProjectRow } from "./ProjectRow";

interface SortableProjectItemProps {
  project: ProjectEntry;
  isActive: boolean;
  isExpanded: boolean;
  isDragging: boolean;
  isLoaded: boolean;
  isLoading: boolean;
  hasRunningConversation: boolean;
  conversations: ConversationSummary[];
  visibleConversations: ConversationSummary[];
  activeConversationId?: string | null;
  showingArchivedConversations: boolean;
  onProjectClick: () => void;
  onCreateConversation: () => void;
  onToggleShowArchivedConversations: () => void;
  onSelectConversation: (projectPath: string, conversationId: string) => void | Promise<void>;
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

export function SortableProjectItem({
  project,
  isActive,
  isExpanded,
  isDragging,
  isLoaded,
  isLoading,
  hasRunningConversation,
  conversations,
  visibleConversations,
  activeConversationId,
  showingArchivedConversations,
  onProjectClick,
  onCreateConversation,
  onToggleShowArchivedConversations,
  onSelectConversation,
  onRemoveProject,
  onRenameProject,
  onArchiveProject,
  onUnarchiveProject,
  onRemoveConversation,
  onRenameConversation,
  onArchiveConversation,
  onUnarchiveConversation,
  onSetConversationPinned,
}: SortableProjectItemProps): ReactNode {
  const { attributes, listeners, setNodeRef, transform, transition } = useSortable({
    id: project.path,
  });

  const style: CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={isDragging ? "z-20 opacity-70" : undefined}
      {...attributes}
    >
      <div className="mb-2">
        <ProjectRow
          projectPath={project.path}
          projectLabel={projectDisplayName(project)}
          isActive={isActive}
          expanded={isExpanded}
          isArchived={Boolean(project.archivedAt)}
          hasConversations={conversations.length > 0}
          hasRunningConversation={hasRunningConversation}
          showingArchivedConversations={showingArchivedConversations}
          dragHandleProps={listeners}
          onProjectClick={onProjectClick}
          onCreateConversation={onCreateConversation}
          onToggleShowArchivedConversations={onToggleShowArchivedConversations}
          onRemoveProject={onRemoveProject ? () => { onRemoveProject(project.path); } : undefined}
          onRenameProject={onRenameProject ? (name) => { onRenameProject(project.path, name); } : undefined}
          onArchiveProject={onArchiveProject ? () => { onArchiveProject(project.path); } : undefined}
          onUnarchiveProject={onUnarchiveProject ? () => { onUnarchiveProject(project.path); } : undefined}
        />

        {isExpanded && (
          <div className="mt-1 space-y-1">
            {!isLoaded && isLoading && (
              <p className="px-3 py-2 text-xs text-fg-faint">Loading conversations...</p>
            )}
            {isLoaded && visibleConversations.length === 0 && (
              <p className="px-3 py-2 text-xs text-fg-faint">
                {conversations.length === 0 ? "No conversations yet" : "Archived conversations hidden"}
              </p>
            )}
            {visibleConversations.map((conversation) => (
              <ConversationRow
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
    </div>
  );
}
