import type { ReactNode } from "react";
import { FolderOpen, Trash2 } from "lucide-react";
import type { ProjectEntry } from "../../types";

interface ProjectSettingsRowProps {
  project: ProjectEntry;
  onRemove: (projectPath: string) => void;
}

/** Single project row inside the Projects settings tab. */
export function ProjectSettingsRow({ project, onRemove }: ProjectSettingsRowProps): ReactNode {
  const ownerSegment = project.path.split("/").slice(-2, -1)[0] ?? "";

  return (
    <div className="flex items-center gap-3 rounded-lg border border-edge bg-panel px-4 py-3">
      <FolderOpen size={18} className="shrink-0 text-fg-muted" />
      <div className="min-w-0 flex-1">
        <span className="text-sm font-medium text-fg">{project.name}</span>
        {ownerSegment && (
          <span className="ml-2 text-xs text-fg-muted">{ownerSegment}</span>
        )}
      </div>
      <button
        type="button"
        onClick={() => onRemove(project.path)}
        className="rounded p-1.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
        title="Remove project"
      >
        <Trash2 size={14} strokeWidth={2} />
      </button>
    </div>
  );
}
