import type { ReactNode } from "react";

interface PipelineControlBarProps {
  statusLabel: string;
  statusClassName: string;
  iterationText: string;
  elapsedText: string;
  isPaused: boolean;
  showPause: boolean;
  showResume: boolean;
  onPause?: () => void;
  onResume?: () => void;
  onCancel: () => void;
}

/** Shared pause/resume/cancel control bar for active pipeline runs. */
export function PipelineControlBar({
  statusLabel,
  statusClassName,
  iterationText,
  elapsedText,
  isPaused,
  showPause,
  showResume,
  onPause,
  onResume,
  onCancel,
}: PipelineControlBarProps): ReactNode {
  return (
    <div className="flex w-full items-center gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
      <div className="flex items-center gap-2 flex-1">
        {isPaused ? (
          <div className="h-3.5 w-3.5 rounded-full border-2 border-[#3b82f6]" />
        ) : (
          <svg className={`h-3.5 w-3.5 animate-spin ${statusClassName}`} xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
        )}
        <span className="text-sm text-[#9898b0]">{statusLabel}... | {iterationText} | {elapsedText}</span>
      </div>
      {showPause && onPause && (
        <button
          onClick={onPause}
          className="shrink-0 rounded-lg bg-[#2563eb] p-2 text-white hover:bg-[#3b82f6] transition-colors"
          title="Pause pipeline"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="5" width="4" height="14" rx="1" />
            <rect x="14" y="5" width="4" height="14" rx="1" />
          </svg>
        </button>
      )}
      {showResume && onResume && (
        <button
          onClick={onResume}
          className="shrink-0 rounded-lg bg-[#22c55e] p-2 text-white hover:bg-[#16a34a] transition-colors"
          title="Resume pipeline"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
        </button>
      )}
      <button
        onClick={onCancel}
        className="shrink-0 rounded-lg bg-[#ef4444] p-2 text-white hover:bg-red-400 transition-colors"
        title="Cancel pipeline"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  );
}
