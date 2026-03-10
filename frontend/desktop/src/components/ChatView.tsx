import type { ReactNode } from "react";
import { useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PipelineRun, RunOptions, CliHealth, AppSettings } from "../types";
import { useToast } from "./shared/Toast";
import { isActive, isTerminal, statusInfo } from "../utils/statusHelpers";
import { resolveAuditedPlanText, resolvePlanText } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { StageCard } from "./shared/StageCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";
import { ArtifactCard } from "./shared/ArtifactCard";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { StageInputOutputCard } from "./shared/StageInputOutputCard";
import { PromptInputBar } from "./shared/PromptInputBar";

const EXCLUDED_ARTIFACT_KINDS = new Set(["result", "executive_summary", "judge", "review", "workspace_context", "enhanced_prompt", "plan", "plan_audit", "plan_final"]);
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
  const recentTerminalRef = useRef<HTMLPreElement>(null);
  const toast = useToast();
  const totalStageLogLines = useMemo(() => Object.values(stageLogs).reduce((sum, lines) => sum + lines.length, 0), [stageLogs]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [run.iterations.length, Object.keys(artifacts).length, totalStageLogLines]);

  const { label: statusLabel, colour: statusColour } = statusInfo(run.status);
  const allStages = run.iterations.flatMap((iter) => iter.stages);
  const enhancedPrompt = artifacts["enhanced_prompt"];
  const enhancedPromptInput = (enhancedPrompt ?? run.prompt).trim();
  const planArtifact = artifacts["plan"];
  const planAuditArtifact = artifacts["plan_final"] ?? artifacts["plan_audit"];
  const planInputForAudit = resolvePlanText(planArtifact);
  const latestCompletedPlanIndex = allStages.reduce(
    (latest, stage, idx) => (stage.stage === "plan" && stage.status === "completed" ? idx : latest),
    -1,
  );
  const latestCompletedPlanAuditIndex = allStages.reduce(
    (latest, stage, idx) => (stage.stage === "plan_audit" && stage.status === "completed" ? idx : latest),
    -1,
  );
  const otherArtifacts = Object.entries(artifacts).filter(([kind]) => !EXCLUDED_ARTIFACT_KINDS.has(kind) && kind !== "diff" && !kind.startsWith("diff_"));
  const headerTitle = run.prompt.length > 60 ? `${run.prompt.slice(0, 60)}...` : run.prompt;
  const isPaused = run.status === "paused";
  const activeStage = run.currentStage;
  const recentTerminalStage = activeStage ?? [...allStages].reverse().map((stage) => stage.stage).find((stage) => (stageLogs[stage]?.length ?? 0) > 0);
  const recentTerminalLines = recentTerminalStage ? (stageLogs[recentTerminalStage] ?? []).slice(-160) : [];
  const recentTerminalLabel = recentTerminalStage?.replace(/_/g, " ");
  useEffect(() => {
    const el = recentTerminalRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [recentTerminalStage, recentTerminalLines.length]);

  return (
    <div className="flex h-full min-h-0 flex-col bg-[#0f0f14]">
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
        <h2 className="text-sm font-medium text-[#e4e4ed] truncate">{headerTitle}</h2>
        <span
          className="ml-auto rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider"
          style={{ color: statusColour, background: `${statusColour}1a` }}
        >
          {statusLabel}
        </span>
      </div>

      <div ref={scrollRef} className="app-scrollbar min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-28 [scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-2xl flex flex-col gap-3">
          <div className="flex justify-end">
            <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
              {run.prompt}
            </div>
          </div>

          <PromptReceivedCard prompt={run.prompt} />

          {allStages.filter((stage) => stage.stage !== "diff_after_coder" && stage.stage !== "diff_after_code_fixer").map((stage, idx) => (
            <div key={`${stage.stage}-${idx}`} className="flex flex-col gap-2">
              {stage.stage === "prompt_enhance" && stage.status === "completed" ? (
                <StageInputOutputCard
                  title="Enhancing Prompt"
                  inputSections={[
                    { label: "Original Prompt", content: run.prompt },
                  ]}
                  outputLabel="Result"
                  outputContent={(enhancedPrompt ?? stage.output).trim() || "No valid enhanced prompt output generated."}
                  modelLabel={stageModelLabel("prompt_enhance", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-emerald-400/25"
                  outputClassName="border border-emerald-400/20 bg-emerald-400/5 text-[#e4e4ed]"
                  terminalLogs={stageLogs[stage.stage]}
                />
              ) : stage.stage === "plan" && stage.status === "completed" && idx === latestCompletedPlanIndex ? (
                <StageInputOutputCard
                  title="Planning"
                  inputSections={[
                    { label: "Original Prompt", content: run.prompt },
                    { label: "Enhanced Prompt", content: enhancedPromptInput },
                  ]}
                  outputLabel="Plan"
                  outputContent={resolvePlanText(planArtifact, stage.output) || "No valid plan output generated."}
                  modelLabel={stageModelLabel("plan", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-sky-400/25"
                  terminalLogs={stageLogs[stage.stage]}
                />
              ) : stage.stage === "plan_audit" && stage.status === "completed" && idx === latestCompletedPlanAuditIndex ? (
                <StageInputOutputCard
                  title="Auditing Plan"
                  inputSections={[
                    { label: "Original Prompt", content: run.prompt },
                    { label: "Enhanced Prompt", content: enhancedPromptInput },
                    { label: "Plan", content: planInputForAudit },
                  ]}
                  outputLabel="Audited Plan"
                  outputContent={resolveAuditedPlanText(planAuditArtifact, stage.output) || "No valid audited plan output generated."}
                  modelLabel={stageModelLabel("plan_audit", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-amber-400/25"
                  outputClassName="border border-amber-400/20 bg-amber-400/5 text-[#e4e4ed]"
                  terminalLogs={stageLogs[stage.stage]}
                />
              ) : stage.stage === "code_reviewer" && stage.status === "completed" ? (
                <StageInputOutputCard
                  title="Code Review"
                  inputSections={[
                    { label: "Original Prompt", content: run.prompt },
                    { label: "Enhanced Prompt", content: enhancedPromptInput },
                  ]}
                  outputLabel="Review Findings"
                  outputContent={artifacts["review"] ?? stage.output ?? "No review output generated."}
                  modelLabel={stageModelLabel("code_reviewer", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-orange-400/25"
                  outputClassName="border border-orange-400/20 bg-orange-400/5 text-[#e4e4ed]"
                  terminalLogs={stageLogs[stage.stage]}
                />
              ) : (
                <StageCard
                  stage={stage}
                  modelLabel={stageModelLabel(stage.stage, settings)}
                  logs={stageLogs[stage.stage]}
                  startedAt={run.status === "running" && run.currentStage === stage.stage && stage.status === "running"
                    ? run.stageStartedAt
                    : undefined}
                />
              )}
            </div>
          ))}

          {isActive(run.status) && run.currentStage && run.currentStage !== "plan_audit" && (
            <ThinkingIndicator stage={run.currentStage} startedAt={run.stageStartedAt} />
          )}

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

          {otherArtifacts.length > 0 && (
            <div className="flex flex-col gap-2">
              {otherArtifacts.map(([kind, content]) => (
                <ArtifactCard key={kind} kind={kind} content={content} />
              ))}
            </div>
          )}
        </div>
      </div>

      <div className="flex w-full max-w-2xl mx-auto flex-col gap-2 px-6 pb-6 pt-2">
        {(isActive(run.status) || isPaused) && (
          <details className="w-full rounded-xl border border-[#2e2e48] bg-[#14141e]">
            <summary className="cursor-pointer select-none px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-[#9898b0] hover:text-[#e4e4ed] transition-colors">
              Recent Terminal{recentTerminalLabel ? ` - ${recentTerminalLabel}` : ""}
            </summary>
            <div className="border-t border-[#2e2e48] p-3">
              <pre ref={recentTerminalRef} className="max-h-56 overflow-auto rounded bg-[#0f0f14] p-2 text-[11px] leading-relaxed text-[#e4e4ed] whitespace-pre-wrap break-words">
                {recentTerminalLines.length > 0 ? recentTerminalLines.join("\n") : "Waiting for terminal output..."}
              </pre>
            </div>
          </details>
        )}

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

        <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
          <span className="truncate" title={run.workspacePath}>{run.workspacePath}</span>
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
