import type { ReactNode } from "react";
import { useState, useRef, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { WorkspaceInfo, ProjectSummary, AgentBackend, RunOptions, CliHealth } from "../types";
import { CLI_MODEL_OPTIONS } from "../types";
import { BACKEND_OPTIONS } from "./shared/constants";
import { PopoverSelect } from "./shared/PopoverSelect";

interface IdleViewProps {
  workspace: WorkspaceInfo | null;
  workspacePath?: string;
  projects: ProjectSummary[];
  cliHealth: CliHealth | null;
  onSelectProject: (projectPath: string) => void | Promise<void>;
  onAddProject: () => void;
  onRun: (options: RunOptions) => void;
}

/** Centred idle landing screen with logo, heading, workspace selector, and input bar. */
export function IdleView({
  workspace,
  workspacePath,
  projects,
  cliHealth,
  onSelectProject,
  onAddProject,
  onRun,
}: IdleViewProps): ReactNode {
  const [prompt, setPrompt] = useState<string>("");
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const [directTask, setDirectTask] = useState(false);
  const [noPlan, setNoPlan] = useState(false);
  const [directAgent, setDirectAgent] = useState<AgentBackend>("claude");
  const [directModel, setDirectModel] = useState<string>("sonnet");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const availableBackends = BACKEND_OPTIONS.filter(
    (opt) => cliHealth?.[opt.value]?.available,
  );

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

  // Close project dropdown on outside click
  useEffect(() => {
    if (!dropdownOpen) return;
    function handleClick(e: MouseEvent): void {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [dropdownOpen]);

  // Close project dropdown on Escape
  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === "Escape") setDropdownOpen(false);
  }, []);
  useEffect(() => {
    if (!dropdownOpen) return;
    document.addEventListener("keydown", handleEscape);
    return () => document.removeEventListener("keydown", handleEscape);
  }, [dropdownOpen, handleEscape]);

  function handleSubmit(): void {
    if (canRun) {
      onRun({
        prompt,
        directTask,
        directTaskAgent: directTask ? directAgent : undefined,
        directTaskModel: directTask ? directModel : undefined,
        noPlan,
      });
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  function projectDisplayName(project: ProjectSummary): string {
    if (project.name.trim().length > 0) return project.name;
    const parts = project.path.split(/[/\\]+/);
    return parts[parts.length - 1] || project.path;
  }

  /** Extract the folder name from the active workspace path. */
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

        {/* Workspace selector dropdown */}
        <div ref={dropdownRef} className="relative">
          <button
            onClick={() => setDropdownOpen((prev) => !prev)}
            className="flex items-center gap-2 text-lg text-[#9898b0] hover:text-[#e4e4ed] transition-colors"
          >
            <span>{workspace ? workspaceName() : "Select a project..."}</span>
            <svg
              className={`h-4 w-4 shrink-0 transition-transform ${dropdownOpen ? "rotate-180" : ""}`}
              xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
            >
              <polyline points="6 9 12 15 18 9" />
            </svg>
          </button>

          {dropdownOpen && (
            <div className="absolute top-full left-1/2 z-50 mt-2 w-64 -translate-x-1/2 rounded-lg border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
              {projects.length === 0 && (
                <span className="block px-3 py-2 text-xs text-[#6b6b80]">No projects yet</span>
              )}

              {projects.map((project) => {
                const isActive = project.path === workspace?.path;
                return (
                  <button
                    key={project.id}
                    onClick={() => {
                      void onSelectProject(project.path);
                      setDropdownOpen(false);
                    }}
                    className={`flex w-full items-center gap-2 px-3 py-2 text-sm transition-colors ${
                      isActive
                        ? "bg-[#24243a] text-[#e4e4ed]"
                        : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                    }`}
                    title={project.path}
                  >
                    {isActive ? (
                      <svg className="h-4 w-4 shrink-0" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M2.5 6L5 8.5L9.5 3.5" />
                      </svg>
                    ) : (
                      <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                      </svg>
                    )}
                    <span className="truncate">{projectDisplayName(project)}</span>
                  </button>
                );
              })}

              {/* Divider + Add project */}
              <div className="my-1 border-t border-[#2e2e48]" />
              <button
                onClick={() => {
                  onAddProject();
                  setDropdownOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-sm text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
              >
                <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
                  <line x1="12" y1="11" x2="12" y2="17" />
                  <line x1="9" y1="14" x2="15" y2="14" />
                </svg>
                <span>Add project</span>
              </button>
            </div>
          )}
        </div>

        {/* Git badge */}
        {workspace && (
          <div className="flex gap-2 text-xs text-[#9898b0]">
            {workspace.isGitRepo && (
              <span className="rounded bg-[#24243a] px-2 py-0.5">
                {workspace.branch ?? "git"}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Bottom section — input + workspace */}
      <div className="flex w-full max-w-2xl flex-col gap-2 px-6 pb-8">
        {/* Input bar with checkboxes inside */}
        <div className="flex w-full flex-col rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3 gap-2">
          {/* Top row: textarea + submit */}
          <div className="flex items-center gap-2">
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
              title={directTask ? "Run direct task" : "Run pipeline"}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="19" x2="12" y2="5" />
                <polyline points="5 12 12 5 19 12" />
              </svg>
            </button>
          </div>

          {/* Bottom row inside box: checkboxes left, dropdowns right */}
          <div className="flex items-center gap-4 border-t border-[#2e2e48] pt-2">
            <label className="flex items-center gap-1.5 text-xs text-[#9898b0] cursor-pointer select-none">
              <input
                type="checkbox"
                checked={directTask}
                onChange={(e) => {
                  const checked = e.target.checked;
                  setDirectTask(checked);
                  if (checked && availableBackends.length > 0) {
                    const first = availableBackends[0].value;
                    setDirectAgent(first);
                    const opts = CLI_MODEL_OPTIONS[first];
                    if (opts && opts.length > 0) setDirectModel(opts[0].value);
                  }
                }}
                className="h-3.5 w-3.5 rounded border-[#2e2e48] bg-[#1a1a24] accent-[#6366f1]"
              />
              Direct Task
            </label>
            <label className="flex items-center gap-1.5 text-xs text-[#9898b0] cursor-pointer select-none">
              <input
                type="checkbox"
                checked={noPlan}
                onChange={(e) => setNoPlan(e.target.checked)}
                disabled={directTask}
                className="h-3.5 w-3.5 rounded border-[#2e2e48] bg-[#1a1a24] accent-[#6366f1] disabled:opacity-30"
              />
              <span className={directTask ? "opacity-30" : ""}>No Plan</span>
            </label>
            {directTask && (
              <div className="ml-auto flex items-center gap-2">
                {/* Agent dropdown (opens upward) */}
                <div ref={agentDropdownRef} className="relative">
                  <button
                    onClick={() => { setAgentDropdownOpen((p) => !p); setModelDropdownOpen(false); }}
                    className="flex items-center gap-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-2 py-1 text-xs text-[#e4e4ed] hover:border-[#6366f1] transition-colors"
                  >
                    <span>{availableBackends.find((b) => b.value === directAgent)?.label ?? directAgent}</span>
                    <svg className="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="6 9 12 15 18 9" /></svg>
                  </button>
                  {agentDropdownOpen && (
                    <div className="absolute bottom-full left-0 z-50 mb-1 min-w-full rounded-lg border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
                      {availableBackends.map((opt) => (
                        <button
                          key={opt.value}
                          onClick={() => {
                            const backend = opt.value;
                            setDirectAgent(backend);
                            const opts = CLI_MODEL_OPTIONS[backend];
                            if (opts && opts.length > 0) setDirectModel(opts[0].value);
                            setAgentDropdownOpen(false);
                          }}
                          className={`flex w-full items-center px-3 py-1.5 text-xs whitespace-nowrap transition-colors ${
                            opt.value === directAgent ? "bg-[#24243a] text-[#e4e4ed]" : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                          }`}
                        >{opt.label}</button>
                      ))}
                    </div>
                  )}
                </div>
                {/* Model dropdown (opens upward) */}
                <div ref={modelDropdownRef} className="relative">
                  <button
                    onClick={() => { setModelDropdownOpen((p) => !p); setAgentDropdownOpen(false); }}
                    className="flex items-center gap-1 rounded border border-[#2e2e48] bg-[#1a1a24] px-2 py-1 text-xs text-[#e4e4ed] hover:border-[#6366f1] transition-colors"
                  >
                    <span>{(CLI_MODEL_OPTIONS[directAgent] ?? []).find((m) => m.value === directModel)?.label ?? directModel}</span>
                    <svg className="h-3 w-3 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5"><polyline points="6 9 12 15 18 9" /></svg>
                  </button>
                  {modelDropdownOpen && (
                    <div className="absolute bottom-full right-0 z-50 mb-1 min-w-full rounded-lg border border-[#2e2e48] bg-[#1a1a2e] py-1 shadow-lg">
                      {(CLI_MODEL_OPTIONS[directAgent] ?? []).map((opt) => (
                        <button
                          key={opt.value}
                          onClick={() => { setDirectModel(opt.value); setModelDropdownOpen(false); }}
                          className={`flex w-full items-center px-3 py-1.5 text-xs whitespace-nowrap transition-colors ${
                            opt.value === directModel ? "bg-[#24243a] text-[#e4e4ed]" : "text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed]"
                          }`}
                        >{opt.label}</button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Workspace path + Open in VS Code */}
        <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
          <span className="truncate" title={workspacePath ?? "No project selected"}>
            {workspacePath ?? "No project selected"}
          </span>
          {workspacePath && (
            <button
              onClick={() => {
                void invoke("open_in_vscode", { path: workspacePath });
              }}
              className="ml-4 flex shrink-0 items-center gap-1.5 rounded px-2 py-0.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
              title="Open in VS Code"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M16 3l5 3v12l-5 3L2 12l5-3" />
                <path d="M16 3L7 12l9 9" />
                <path d="M16 3v18" />
              </svg>
              Open in VS Code
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
