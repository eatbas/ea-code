import type { ReactNode } from "react";
import { Trash2 } from "lucide-react";

interface QueuedPromptBannerProps {
  /** The queued prompt text. */
  prompt: string;
  /** Called when the user clicks the trash button to clear the queue. */
  onDelete: () => void;
}

export function QueuedPromptBanner({
  prompt,
  onDelete,
}: QueuedPromptBannerProps): ReactNode {

  return (
    <div className="relative overflow-hidden border-b border-edge">
      {/* Green flowing light — same as PipelineStatusBar */}
      <div className="absolute inset-x-0 top-0 h-[2px]">
        <div className="h-full w-1/3 animate-[flowRight_2s_ease-in-out_infinite] rounded-full bg-gradient-to-r from-transparent via-running-dot to-transparent" />
      </div>

      <div className="flex items-center gap-2.5 px-4 py-2">
        {/* Running indicator */}
        <span className="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-running-dot" />
        <span className="text-[11px] font-semibold text-running-dot">
          Queued
        </span>

        {/* Truncated prompt */}
        <span className="min-w-0 flex-1 truncate text-xs text-fg-muted">
          {prompt}
        </span>

        {/* Delete queued prompt */}
        <button
          type="button"
          onClick={onDelete}
          className="inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-fg-faint transition-colors hover:bg-elevated hover:text-danger"
          title="Remove queued message"
        >
          <Trash2 size={12} />
        </button>
      </div>
    </div>
  );
}
