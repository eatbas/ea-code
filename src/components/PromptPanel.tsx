import type { ReactNode } from "react";
import { useState } from "react";
import type { WorkspaceInfo } from "../types";

interface PromptPanelProps {
  onRun: (prompt: string) => void;
  onCancel: () => void;
  onSelectFolder: () => void;
  workspace: WorkspaceInfo | null;
  isRunning: boolean;
  currentIteration: number;
  maxIterations: number;
}

/** Left sidebar panel for entering prompts and controlling pipeline execution. */
export function PromptPanel({
  onRun,
  onCancel,
  onSelectFolder,
  workspace,
  isRunning,
  currentIteration,
  maxIterations,
}: PromptPanelProps): ReactNode {
  const [prompt, setPrompt] = useState<string>("");

  const canRun = prompt.trim().length > 0 && workspace !== null && !isRunning;

  function handleSubmit(): void {
    if (canRun) {
      onRun(prompt);
    }
  }

  return (
    <div className="flex flex-col gap-3 p-4">
      <textarea
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="Enter your development prompt..."
        rows={6}
        className="w-full resize-y rounded border border-[#2e2e48] bg-[#0f0f14] p-3 text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:border-[#6366f1] focus:outline-none"
      />

      <button
        onClick={onSelectFolder}
        className="flex items-center gap-2 rounded border border-[#2e2e48] bg-[#24243a] px-3 py-2 text-sm text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed] transition-colors text-left truncate"
      >
        {/* Folder icon */}
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="shrink-0"
        >
          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
        </svg>
        <span className="truncate">
          {workspace ? workspace.path : "Select workspace..."}
        </span>
      </button>

      {workspace && (
        <div className="flex gap-2 text-xs text-[#9898b0]">
          {workspace.isGitRepo && (
            <span className="rounded bg-[#24243a] px-2 py-0.5">
              {workspace.branch ?? "git"}
            </span>
          )}
          {workspace.isDirty && (
            <span className="rounded bg-[#24243a] px-2 py-0.5 text-[#f59e0b]">
              dirty
            </span>
          )}
        </div>
      )}

      <button
        onClick={handleSubmit}
        disabled={!canRun}
        className="rounded bg-[#6366f1] px-4 py-2 text-sm font-medium text-white hover:bg-[#818cf8] disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      >
        Run Pipeline
      </button>

      {isRunning && (
        <>
          <button
            onClick={onCancel}
            className="rounded bg-[#ef4444] px-4 py-2 text-sm font-medium text-white hover:bg-red-400 transition-colors"
          >
            Cancel
          </button>

          <p className="text-center text-xs text-[#9898b0]">
            Iteration {currentIteration} / {maxIterations}
          </p>
        </>
      )}
    </div>
  );
}
