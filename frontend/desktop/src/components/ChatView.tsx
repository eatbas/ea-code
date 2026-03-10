import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PipelineRun, RunOptions, CliHealth, AppSettings } from "../types";
import { useToast } from "./shared/Toast";
import { isActive, isTerminal, statusInfo } from "../utils/statusHelpers";
import { StageCard } from "./shared/StageCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";
import { ArtifactCard } from "./shared/ArtifactCard";
import { PromptCard } from "./shared/PromptCard";
import { FinalPlanCard } from "./shared/FinalPlanCard";
import { PromptInputBar } from "./shared/PromptInputBar";

/** Artefact kinds consumed by specialised cards — excluded from the generic list. */
const EXCLUDED_ARTIFACT_KINDS = new Set([
  "result", "executive_summary", "judge",
  "workspace_context", "enhanced_prompt",
  "plan", "plan_audit", "plan_final",
]);

interface ChatViewProps {
  run: PipelineRun;
  stageLogs: Record<string, string[]>;
  artifacts: Record<string, string>;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup: () => void;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  onNewSession: () => void;
  onContinue: (options: RunOptions) => void;
}

/** Chat-style running view with stage timeline, result card, and follow-up input. */
export function ChatView({
  run,
  stageLogs,
  artifacts,
  cliHealth,
  settings,
  onMissingAgentSetup,
  onPause,
  onResume,
  onCancel,
  onNewSession,
  onContinue,
}: ChatViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);
  const [promptOpen, setPromptOpen] = useState(false);
  const toast = useToast();

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [run.iterations.length, Object.keys(artifacts).length, Object.keys(stageLogs).length]);

  const { label: statusLabel, colour: statusColour } = statusInfo(run.status);

  const allStages = run.iterations.flatMap((iter) => iter.stages);

  const enhancedPrompt = artifacts["enhanced_prompt"];
  const planArtifact = artifacts["plan"];
  const planAuditArtifact = artifacts["plan_audit"] ?? artifacts["plan_final"];

  const otherArtifacts = Object.entries(artifacts).filter(
    ([kind]) => !EXCLUDED_ARTIFACT_KINDS.has(kind),
  );

  const headerTitle = run.prompt.length > 60
    ? `${run.prompt.slice(0, 60)}...`
    : run.prompt;
  const isPaused = run.status === "paused";

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      {/* Header — matches SessionDetailView style */}
      <div className="flex items-center gap-3 border-b border-[#2e2e48] px-6 py-3">
        <button
          onClick={onNewSession}
          className="rounded p-1 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
          title="Back to home"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="19" y1="12" x2="5" y2="12" />
            <polyline points="12 19 5 12 12 5" />
          </svg>
        </button>
        <h2 className="text-sm font-medium text-[#e4e4ed] truncate">
          {headerTitle}
        </h2>
        <span
          className="ml-auto rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider"
          style={{ color: statusColour, background: `${statusColour}1a` }}
        >
          {statusLabel}
        </span>
      </div>

      {/* Scrollable timeline area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <div className="mx-auto max-w-2xl flex flex-col gap-3">
          {/* User prompt — right-aligned bubble */}
          <div className="flex justify-end">
            <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
              {run.prompt}
            </div>
          </div>

          {/* Prompt received — collapsible card matching stage card style */}
          <article
            className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden cursor-pointer"
            onClick={() => setPromptOpen((prev) => !prev)}
          >
            <div className="flex items-center gap-2 px-3 py-2 hover:bg-[#1a1a2a] transition-colors">
              <svg
                className={`h-3 w-3 text-[#9898b0] transition-transform ${promptOpen ? "rotate-90" : ""}`}
                viewBox="0 0 24 24"
                fill="currentColor"
              >
                <path d="M8 5v14l11-7z" />
              </svg>
              <span
                className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed]"
                style={{ background: "rgba(34, 197, 94, 0.22)" }}
              >
                Prompt Received
              </span>
              <span className="ml-auto rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
                Completed
              </span>
            </div>
            {promptOpen && (
              <div className="px-3 pb-3">
                <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
                  {run.prompt}
                </div>
              </div>
            )}
          </article>

          {/* Stage timeline cards with inline prompt displays */}
          {allStages.map((stage, idx) => (
            <div key={`${stage.stage}-${idx}`} className="flex flex-col gap-2">
              <StageCard stage={stage} logs={stageLogs[stage.stage]} />

              {/* After PROMPT stage — show the original prompt */}
              {stage.stage === "prompt_enhance" && stage.status === "completed" && !enhancedPrompt && (
                <div className="rounded-lg border border-[#2e2e48] bg-[#14141e] px-3 py-2">
                  <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                    Original Prompt
                  </span>
                  <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
                    {run.prompt}
                  </div>
                </div>
              )}

              {/* After PROMPT stage when enhanced prompt is ready — show both */}
              {stage.stage === "prompt_enhance" && stage.status === "completed" && enhancedPrompt && (
                <PromptCard originalPrompt={run.prompt} enhancedPrompt={enhancedPrompt} durationMs={stage.durationMs} />
              )}

              {/* After plan_audit — show final plan (plan + audit combined) */}
              {stage.stage === "plan_audit" && stage.status === "completed" && (planArtifact || planAuditArtifact) && (
                <FinalPlanCard
                  plannerPlan={planArtifact}
                  auditedPlan={planAuditArtifact}
                  durationMs={
                    allStages
                      .filter((s) => s.stage === "plan" || s.stage === "plan_audit")
                      .reduce((sum, s) => sum + s.durationMs, 0)
                  }
                />
              )}
            </div>
          ))}

          {/* Thinking indicator for currently running stage */}
          {isActive(run.status) && run.currentStage && (
            <ThinkingIndicator stage={run.currentStage} startedAt={run.stageStartedAt} />
          )}

          {/* Result card when pipeline reaches terminal state */}
          {isTerminal(run.status) && (
            <ResultCard
              status={run.status}
              finalVerdict={run.finalVerdict}
              iterationCount={run.currentIteration || run.iterations.length}
              totalDurationMs={computeDuration(run.startedAt, run.completedAt)}
              completedAt={run.completedAt}
              executiveSummary={artifacts["executive_summary"]}
              error={run.error}
              stageRows={buildStageRows(allStages)}
              artifacts={artifacts}
            />
          )}

          {/* Other artefacts (diff, plan, review) as collapsible cards */}
          {otherArtifacts.length > 0 && (
            <div className="flex flex-col gap-2">
              {otherArtifacts.map(([kind, content]) => (
                <ArtifactCard key={kind} kind={kind} content={content} />
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Bottom bar */}
      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        {(isActive(run.status) || isPaused) && (
          <div className="flex w-full items-center gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
            <div className="flex items-center gap-2 flex-1">
              {isPaused ? (
                <div className="h-3.5 w-3.5 rounded-full border-2 border-[#3b82f6]" />
              ) : (
                <svg className="animate-spin h-3.5 w-3.5" style={{ color: statusColour }} xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                  <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                </svg>
              )}
              <span className="text-sm text-[#9898b0]">{statusLabel}...</span>
            </div>
            {isActive(run.status) && (
              <button
                onClick={() => onPause()}
                className="shrink-0 rounded-lg bg-[#2563eb] p-2 text-white hover:bg-[#3b82f6] transition-colors"
                title="Pause pipeline"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="5" width="4" height="14" rx="1" />
                  <rect x="14" y="5" width="4" height="14" rx="1" />
                </svg>
              </button>
            )}
            {isPaused && (
              <button
                onClick={() => onResume()}
                className="shrink-0 rounded-lg bg-[#22c55e] p-2 text-white hover:bg-[#16a34a] transition-colors"
                title="Resume pipeline"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </button>
            )}
            <button
              onClick={() => onCancel()}
              className="shrink-0 rounded-lg bg-[#ef4444] p-2 text-white hover:bg-red-400 transition-colors"
              title="Cancel pipeline"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        )}

        {isTerminal(run.status) && (
          <PromptInputBar
            placeholder="Continue this session..."
            cliHealth={cliHealth}
            settings={settings}
            onMissingAgentSetup={onMissingAgentSetup}
            onSubmit={onContinue}
          />
        )}

        {/* Workspace path + Open in VS Code */}
        <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
          <span className="truncate" title={run.workspacePath}>
            {run.workspacePath}
          </span>
          <button
            onClick={() => {
              void invoke("open_in_vscode", { path: run.workspacePath }).catch(() => {
                toast.error("Failed to open VS Code.");
              });
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
        </div>
      </div>
    </div>
  );
}
