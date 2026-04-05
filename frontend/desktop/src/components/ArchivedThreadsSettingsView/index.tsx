import type { ReactNode } from "react";
import { ArchiveRestore, MessageSquare } from "lucide-react";
import type { ArchivedThread } from "../../hooks/useArchivedThreads";

interface ArchivedThreadsSettingsViewProps {
  threads: ArchivedThread[];
  loading: boolean;
  onUnarchive: (projectPath: string, conversationId: string) => Promise<void>;
}

function formatDate(iso: string | null): string {
  if (!iso) return "";
  const date = new Date(iso);
  return date.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: "numeric" });
}

export function ArchivedThreadsSettingsView({ threads, loading, onUnarchive }: ArchivedThreadsSettingsViewProps): ReactNode {
  return (
    <div className="relative flex h-full flex-col bg-surface">
      <div className="flex-1 overflow-y-auto px-8 py-8">
        <div className="mx-auto flex max-w-2xl flex-col gap-6">
          {/* Header */}
          <div className="mb-2">
            <h1 className="text-xl font-bold text-fg">Archived Threads</h1>
            <p className="mt-1 text-sm text-fg-muted">
              View and restore previously archived conversations.
            </p>
          </div>

          {/* Loading */}
          {loading && (
            <p className="py-12 text-center text-sm text-fg-muted">Loading archived threads...</p>
          )}

          {/* Empty state */}
          {!loading && threads.length === 0 && (
            <p className="py-12 text-center text-sm text-fg-muted">No archived threads.</p>
          )}

          {/* Thread list */}
          {!loading && threads.length > 0 && (
            <div className="flex flex-col gap-3">
              {threads.map((t) => (
                <div
                  key={t.conversation.id}
                  className="flex items-center gap-3 rounded-lg border border-edge bg-panel px-4 py-3"
                >
                  <MessageSquare size={18} className="shrink-0 text-fg-muted" />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-fg">
                      {t.conversation.title || "Untitled thread"}
                    </p>
                    <p className="mt-0.5 text-xs text-fg-muted">
                      {t.projectName}
                      {t.conversation.archivedAt && (
                        <span className="ml-2">&middot; archived {formatDate(t.conversation.archivedAt)}</span>
                      )}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => void onUnarchive(t.projectPath, t.conversation.id)}
                    className="flex items-center gap-1.5 rounded-md border border-edge bg-elevated px-3 py-1.5 text-xs font-medium text-fg-muted transition-colors hover:bg-active hover:text-fg"
                    title="Unarchive thread"
                  >
                    <ArchiveRestore size={14} />
                    Unarchive
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
