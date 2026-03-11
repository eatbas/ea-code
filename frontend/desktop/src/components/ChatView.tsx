import type { ReactNode } from "react";
import type { PipelineRun, RunOptions, CliHealth, AppSettings } from "../types";
import { isActive, isTerminal, statusInfo } from "../utils/statusHelpers";
import { resolveAuditedPlanText, resolvePlanText } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { useElapsedTimer } from "../hooks/useElapsedTimer";
import { useRecentTerminal } from "../hooks/useRecentTerminal";
import { StageCard } from "./shared/StageCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";
import { ArtifactCard } from "./shared/ArtifactCard";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { StageInputOutputCard } from "./shared/StageInputOutputCard";
import { PromptInputBar } from "./shared/PromptInputBar";
import { RecentTerminalPanel } from "./shared/RecentTerminalPanel";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";
import { PipelineControlBar } from "./shared/PipelineControlBar";

const EXCLUDED_ARTIFACT_KINDS = new Set(["result", "executive_summary", "judge", "review", "workspace_context", "session_memory", "enhanced_prompt", "plan", "plan_audit", "plan_final"]);

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
  const elapsedText = useElapsedTimer(run.status, run.startedAt, run.completedAt);
  const { label: statusLabel, colour: statusColour } = statusInfo(run.status);
  const allStages = run.iterations.flatMap((iter) => iter.stages);
  const visibleStages = allStages.filter((stage) => stage.stage !== "diff_after_coder" && stage.stage !== "diff_after_code_fixer");
  const terminal = useRecentTerminal(stageLogs, run.currentStage, allStages);

  const enhancedPrompt = artifacts["enhanced_prompt"];
  const enhancedPromptInput = (enhancedPrompt ?? run.prompt).trim();
  const planArtifact = artifacts["plan"];
  const planAuditArtifact = artifacts["plan_final"] ?? artifacts["plan_audit"];
  const planInputForAudit = resolvePlanText(planArtifact);
  const latestCompletedPlanIndex = visibleStages.reduce((latest, stage, idx) => (stage.stage === "plan" && stage.status === "completed" ? idx : latest), -1);
  const latestCompletedPlanAuditIndex = visibleStages.reduce((latest, stage, idx) => (stage.stage === "plan_audit" && stage.status === "completed" ? idx : latest), -1);
  const otherArtifacts = Object.entries(artifacts).filter(([kind]) => !EXCLUDED_ARTIFACT_KINDS.has(kind) && kind !== "diff" && !kind.startsWith("diff_"));
  const headerTitle = run.prompt.length > 60 ? `${run.prompt.slice(0, 60)}...` : run.prompt;
  const isPaused = run.status === "paused";
  const iterationText = `Iteration ${Math.max(1, run.currentIteration)}/${Math.max(1, run.maxIterations)}`;

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
      <div className="app-scrollbar min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-28 [scrollbar-gutter:stable_both-edges]">
        <div className="mx-auto max-w-2xl flex flex-col gap-3">
          <div className="flex justify-end">
            <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
              {run.prompt}
            </div>
          </div>
          <PromptReceivedCard prompt={run.prompt} />
          {visibleStages.map((stage, idx) => (
            <div key={`${stage.stage}-${idx}`} className="flex flex-col gap-2">
              {stage.stage === "prompt_enhance" && stage.status === "completed" ? (
                <StageInputOutputCard
                  title="Enhancing Prompt"
                  inputSections={[{ label: "Original Prompt", content: run.prompt }]}
                  outputLabel="Result"
                  outputContent={(enhancedPrompt ?? stage.output).trim() || "No valid enhanced prompt output generated."}
                  modelLabel={stageModelLabel("prompt_enhance", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-emerald-400/25"
                  outputClassName="border border-emerald-400/20 bg-emerald-400/5 text-[#e4e4ed]"
                />
              ) : stage.stage === "plan" && stage.status === "completed" && idx === latestCompletedPlanIndex ? (
                <StageInputOutputCard
                  title="Planning"
                  inputSections={[{ label: "Original Prompt", content: run.prompt }, { label: "Enhanced Prompt", content: enhancedPromptInput }]}
                  outputLabel="Plan"
                  outputContent={resolvePlanText(planArtifact, stage.output) || "No valid plan output generated."}
                  modelLabel={stageModelLabel("plan", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-sky-400/25"
                />
              ) : stage.stage === "plan_audit" && stage.status === "completed" && idx === latestCompletedPlanAuditIndex ? (
                <StageInputOutputCard
                  title="Auditing Plan"
                  inputSections={[{ label: "Original Prompt", content: run.prompt }, { label: "Enhanced Prompt", content: enhancedPromptInput }, { label: "Plan", content: planInputForAudit }]}
                  outputLabel="Audited Plan"
                  outputContent={resolveAuditedPlanText(planAuditArtifact, stage.output) || "No valid audited plan output generated."}
                  modelLabel={stageModelLabel("plan_audit", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-amber-400/25"
                  outputClassName="border border-amber-400/20 bg-amber-400/5 text-[#e4e4ed]"
                />
              ) : stage.stage === "code_reviewer" && stage.status === "completed" ? (
                <StageInputOutputCard
                  title="Code Review"
                  inputSections={[{ label: "Original Prompt", content: run.prompt }, { label: "Enhanced Prompt", content: enhancedPromptInput }]}
                  outputLabel="Review Findings"
                  outputContent={artifacts["review"] ?? stage.output ?? "No review output generated."}
                  modelLabel={stageModelLabel("code_reviewer", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-orange-400/25"
                  outputClassName="border border-orange-400/20 bg-orange-400/5 text-[#e4e4ed]"
                />
              ) : stage.stage === "judge" && stage.status === "completed" ? (
                <StageInputOutputCard
                  title="Judge"
                  inputSections={[
                    { label: "Original Prompt", content: run.prompt },
                    { label: "Enhanced Prompt", content: enhancedPromptInput },
                    { label: "Plan", content: resolveAuditedPlanText(planAuditArtifact, planArtifact) },
                    { label: "Review Findings", content: [...visibleStages.slice(0, idx)].reverse().find((entry) => entry.stage === "code_reviewer")?.output ?? artifacts["review"] ?? "" },
                    { label: "Fixer Output", content: [...visibleStages.slice(0, idx)].reverse().find((entry) => entry.stage === "code_fixer")?.output ?? "" },
                  ]}
                  outputLabel="Decision"
                  outputContent={artifacts["judge"] ?? stage.output ?? "No judge output generated."}
                  modelLabel={stageModelLabel("judge", settings)}
                  durationMs={stage.durationMs}
                  badgeClassName="bg-rose-400/25"
                  outputClassName="border border-rose-400/20 bg-rose-400/5 text-[#e4e4ed]"
                />
              ) : (
                <StageCard
                  stage={stage}
                  modelLabel={stageModelLabel(stage.stage, settings)}
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
          <RecentTerminalPanel
            label={terminal.label}
            lines={terminal.lines}
            terminalRef={terminal.terminalRef}
          />
        )}
        {(isActive(run.status) || isPaused) && (
          <PipelineControlBar
            statusLabel={statusLabel}
            statusColour={statusColour}
            iterationText={iterationText}
            elapsedText={elapsedText}
            isPaused={isPaused}
            showPause={isActive(run.status)}
            showResume={isPaused}
            onPause={onPause}
            onResume={onResume}
            onCancel={onCancel}
          />
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
        <WorkspaceFooter path={run.workspacePath} />
      </div>
    </div>
  );
}
