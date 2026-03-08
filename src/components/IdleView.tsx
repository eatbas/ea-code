import type { ReactNode } from "react";
import { useState, useRef, useEffect } from "react";
import type { WorkspaceInfo } from "../types";

interface IdleViewProps {
  workspace: WorkspaceInfo | null;
  onSelectFolder: () => void;
  onRun: (prompt: string) => void;
}

/** Centred idle landing screen with logo, heading, workspace selector, and input bar. */
export function IdleView({
  workspace,
  onSelectFolder,
  onRun,
}: IdleViewProps): ReactNode {
  const [prompt, setPrompt] = useState<string>("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const canRun = prompt.trim().length > 0 && workspace !== null;

  /** Auto-resize the textarea to fit its content. */
  function autoResize(): void {
    const el = textareaRef.current;
    if (el) {
      el.style.height = "auto";
      el.style.height = `${Math.min(el.scrollHeight, 160)}px`;
    }
  }

  useEffect(() => {
    autoResize();
  }, [prompt]);

  function handleSubmit(): void {
    if (canRun) {
      onRun(prompt);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  /** Extract the folder name from a full path. */
  function workspaceName(): string {
    if (!workspace) return "";
    const parts = workspace.path.split(/[/\\]+/);
    return parts[parts.length - 1] || workspace.path;
  }

  return (
    <div className="flex h-full flex-col items-center bg-[#0f0f14]">
      {/* Centre content — logo, heading, workspace */}
      <div className="flex flex-1 flex-col items-center justify-center gap-4">
        {/* Logo icon */}
        <div className="flex h-16 w-16 items-center justify-center rounded-2xl border border-[#2e2e48] bg-[#1a1a24]">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#e4e4ed" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="16 18 22 12 16 6" />
            <polyline points="8 6 2 12 8 18" />
          </svg>
        </div>

        {/* Heading */}
        <h1 className="text-3xl font-bold text-[#e4e4ed]">Let's build</h1>

        {/* Workspace selector */}
        <button
          onClick={onSelectFolder}
          className="flex items-center gap-2 text-lg text-[#9898b0] hover:text-[#e4e4ed] transition-colors"
        >
          <span>{workspace ? workspaceName() : "Select a project..."}</span>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <polyline points="6 9 12 15 18 9" />
          </svg>
        </button>

        {/* Git badges */}
        {workspace && (
          <div className="flex gap-2 text-xs text-[#9898b0]">
            {workspace.isGitRepo && (
              <span className="rounded bg-[#24243a] px-2 py-0.5">
                {workspace.branch ?? "git"}
              </span>
            )}
            {workspace.isGitRepo && (
              workspace.isDirty ? (
                <span className="rounded bg-[#24243a] px-2 py-0.5 text-[#f59e0b]">
                  dirty
                </span>
              ) : (
                <span className="rounded bg-[#24243a] px-2 py-0.5 text-[#22c55e]">
                  clean
                </span>
              )
            )}
          </div>
        )}
      </div>

      {/* Bottom section — cards + input */}
      <div className="flex w-full max-w-2xl flex-col items-center gap-5 px-6 pb-8">
        {/* Input bar */}
        <div className="flex w-full items-center gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
          <textarea
            ref={textareaRef}
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="What would you like to build?"
            rows={1}
            className="flex-1 resize-none bg-transparent text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:outline-none"
            style={{ maxHeight: "160px" }}
          />
          <button
            onClick={handleSubmit}
            disabled={!canRun}
            className="shrink-0 rounded-lg bg-[#e4e4ed] p-2 text-[#0f0f14] hover:bg-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="Run pipeline"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="12" y1="19" x2="12" y2="5" />
              <polyline points="5 12 12 5 19 12" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}
