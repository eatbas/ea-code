import type { ReactNode } from "react";
import { useState, useEffect, useRef } from "react";
import type { SessionDetail, RunDetail, RunOptions, AgentBackend, CliHealth } from "../types";
import { CLI_MODEL_OPTIONS } from "../types";
import { BACKEND_OPTIONS } from "./shared/constants";
import { PopoverSelect } from "./shared/PopoverSelect";

interface SessionDetailViewProps {
  sessionDetail: SessionDetail | null;
  loading: boolean;
  cliHealth: CliHealth | null;
  onRun: (options: RunOptions) => void;
  onBackToHome: () => void;
}

/** Displays a session's run history and allows continuing the conversation. */
export function SessionDetailView({
  sessionDetail,
  loading,
  cliHealth,
  onRun,
  onBackToHome,
}: SessionDetailViewProps): ReactNode {
  const [prompt, setPrompt] = useState<string>("");
  const [directTask, setDirectTask] = useState(false);
  const [noPlan, setNoPlan] = useState(false);
  const [directAgent, setDirectAgent] = useState<AgentBackend>("claude");
  const [directModel, setDirectModel] = useState<string>("sonnet");
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const availableBackends = BACKEND_OPTIONS.filter(
    (opt) => cliHealth?.[opt.value]?.available,
  );

  const canRun = prompt.trim().length > 0;

  useEffect(() => {
    const el = textareaRef.current;
    if (el) { el.style.height = "auto"; el.style.height = `${Math.min(el.scrollHeight, 160)}px`; }
  }, [prompt]);

  // Scroll to bottom when session detail loads
  useEffect(() => {
    const el = scrollRef.current;
    if (el) { el.scrollTop = el.scrollHeight; }
  }, [sessionDetail]);

  function handleSubmit(): void {
    if (canRun) {
      onRun({
        prompt,
        directTask,
        directTaskAgent: directTask ? directAgent : undefined,
        directTaskModel: directTask ? directModel : undefined,
        noPlan,
      });
      setPrompt("");
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>): void {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Loading session...</span>
      </div>
    );
  }

  if (!sessionDetail) {
    return (
      <div className="flex h-full items-center justify-center bg-[#0f0f14]">
        <span className="text-sm text-[#9898b0]">Session not found.</span>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      {/* Header */}
      <div className="flex items-center gap-3 border-b border-[#2e2e48] px-6 py-3">
        <button
          onClick={onBackToHome}
          className="rounded p-1 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="Back to home"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
        </button>
        <h2 className="text-sm font-medium text-[#e4e4ed] truncate">
          {sessionDetail.title || "Session"}
        </h2>
        <span className="text-xs text-[#6f7086]">
          {sessionDetail.runs.length} {sessionDetail.runs.length === 1 ? "run" : "runs"}
        </span>
      </div>

      {/* Scrollable run history */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <div className="mx-auto max-w-2xl flex flex-col gap-6">
          {sessionDetail.runs.length === 0 && (
            <div className="text-center text-sm text-[#9898b0] py-8">
              No runs in this session yet. Send a prompt to get started.
            </div>
          )}

          {sessionDetail.runs.map((run) => (
            <RunCard key={run.id} run={run} />
          ))}
        </div>
      </div>

      {/* Bottom input bar */}
      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        <div className="flex w-full flex-col rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3 gap-2">
          <div className="flex items-center gap-2">
            <textarea
              ref={textareaRef}
              value={prompt}
              onChange={(e) => setPrompt(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Continue this session..."
              rows={1}
              className="flex-1 resize-none bg-transparent text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:outline-none"
              style={{ maxHeight: "160px" }}
            />
            <button
              onClick={handleSubmit}
              disabled={!canRun}
              className="shrink-0 rounded-lg bg-[#e4e4ed] p-2 text-[#0f0f14] hover:bg-white disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
              title="Run"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="12" y1="19" x2="12" y2="5" />
                <polyline points="5 12 12 5 19 12" />
              </svg>
            </button>
          </div>

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
                <PopoverSelect
                  value={directAgent}
                  options={availableBackends}
                  direction="up"
                  align="left"
                  onChange={(val) => {
                    const backend = val as AgentBackend;
                    setDirectAgent(backend);
                    const opts = CLI_MODEL_OPTIONS[backend];
                    if (opts && opts.length > 0) setDirectModel(opts[0].value);
                  }}
                />
                <PopoverSelect
                  value={directModel}
                  options={CLI_MODEL_OPTIONS[directAgent] ?? []}
                  direction="up"
                  align="right"
                  onChange={setDirectModel}
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/** Displays a single run as a prompt bubble + result card. */
function RunCard({ run }: { run: RunDetail }): ReactNode {
  const verdictColour = run.finalVerdict === "pass"
    ? "#22c55e"
    : run.finalVerdict === "fail"
      ? "#ef4444"
      : "#9898b0";

  const statusColour = run.status === "completed"
    ? "#22c55e"
    : run.status === "failed"
      ? "#ef4444"
      : run.status === "cancelled"
        ? "#f59e0b"
        : "#9898b0";

  return (
    <div className="flex flex-col gap-3">
      {/* User prompt — right-aligned bubble */}
      <div className="flex justify-end">
        <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
          {run.prompt}
        </div>
      </div>

      {/* Run result — left-aligned */}
      <div className="flex justify-start">
        <div className="w-full rounded-2xl rounded-bl-md border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
          {/* Status row */}
          <div className="flex items-center gap-2 mb-2">
            <div className="h-2 w-2 rounded-full" style={{ backgroundColor: statusColour }} />
            <span className="text-xs font-medium capitalize" style={{ color: statusColour }}>
              {run.status}
            </span>
            {run.finalVerdict && (
              <span className="text-xs px-1.5 py-0.5 rounded" style={{ color: verdictColour, backgroundColor: `${verdictColour}15` }}>
                {run.finalVerdict}
              </span>
            )}
            {run.completedAt && (
              <span className="ml-auto text-xs text-[#6f7086]">
                {formatTimestamp(run.completedAt)}
              </span>
            )}
          </div>

          {/* Executive summary */}
          {run.executiveSummary && (
            <p className="text-sm text-[#c4c4d4] whitespace-pre-wrap leading-relaxed">
              {run.executiveSummary}
            </p>
          )}

          {/* Error message */}
          {run.error && (
            <p className="text-xs text-[#ef4444] mt-2">
              {run.error}
            </p>
          )}

          {/* Iteration count */}
          {run.iterations.length > 0 && (
            <div className="mt-2 text-xs text-[#6f7086]">
              {run.iterations.length} {run.iterations.length === 1 ? "iteration" : "iterations"}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/** Formats an ISO timestamp into a readable date/time string. */
function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) +
      " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  } catch {
    return iso;
  }
}
