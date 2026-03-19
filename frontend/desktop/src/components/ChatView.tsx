import type { ReactNode } from "react";
import type { PipelineRun, PipelineStage, RunOptions, CliHealth, AppSettings } from "../types";
import { isActive, isTerminal, statusInfo, statusToneClasses } from "../utils/statusHelpers";
import { resolveAuditedPlanText, resolvePlanText } from "../utils/formatters";
import { stageModelLabel } from "../utils/stageModelLabels";
import { useElapsedTimer } from "../hooks/useElapsedTimer";
import { useRecentTerminal } from "../hooks/useRecentTerminal";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { ResultCard, buildStageRows, computeDuration } from "./shared/ResultCard";
import { PromptReceivedCard } from "./shared/PromptReceivedCard";
import { StageInputOutputCard } from "./shared/StageInputOutputCard";
import { RichStageCard } from "./shared/RichStageCard";
import { TabbedPlanCard, isPlanStage } from "./shared/TabbedPlanCard";
import { TabbedReviewCard, isReviewStage } from "./shared/TabbedReviewCard";
import { PromptInputBar } from "./shared/PromptInputBar";
import { RecentTerminalPanel } from "./shared/RecentTerminalPanel";
import { WorkspaceFooter } from "./shared/WorkspaceFooter";
import { PipelineControlBar } from "./shared/PipelineControlBar";
import { PlannerProgressRow } from "./shared/PlannerProgressRow";
import { ReviewerProgressRow } from "./shared/ReviewerProgressRow";

/** Artifact kinds that are handled specially and not shown as generic artifact cards. */
const EXCLUDED_ARTIFACT_KINDS = new Set([
  "result",
  "executive_summary",
  "judge",
  "review",
  "workspace_context",
  "session_memory",
  "enhanced_prompt",
  "plan",
  "plan_audit",
  "plan_final",
]);

interface ChatViewProps {
  run: PipelineRun;
  stageLogs: Record<string, string[]>;
  /** Artifacts from live pipeline (only available during active run). */
  artifacts: Record<string, string>;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup: () => void;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
  onBackToHome: () => void;
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
  onBackToHome,
  onContinue,
}: ChatViewProps): ReactNode {
  const elapsedText = useElapsedTimer(run.status, run.startedAt, run.completedAt);
  const { label: statusLabel } = statusInfo(run.status);
  const statusClasses = statusToneClasses(run.status);
  const allStages = run.iterations.flatMap((iter) => iter.stages);
  const visibleStages = allStages;
  const terminal = useRecentTerminal(stageLogs, run.currentStage, allStages);

  const enhancedPrompt = artifacts["enhanced_prompt"];
  const enhancedPromptInput = (enhancedPrompt ?? run.prompt).trim();
  const planArtifact = artifacts["plan"];
  const planAuditArtifact = artifacts["plan_final"] ?? artifacts["plan_audit"];
  const planInputForAudit = resolvePlanText(planArtifact);
  const latestCompletedPlanAuditIndex = visibleStages.reduce((latest, stage, idx) => (stage.stage === "plan_audit" && stage.status === "completed" ? idx : latest), -1);
  const otherArtifacts = Object.entries(artifacts).filter(
    ([kind]) => !EXCLUDED_ARTIFACT_KINDS.has(kind) && !kind.startsWith("diff_") && !kind.startsWith("plan_") && !kind.startsWith("review_"),
  );

  // Group all plan stages for the tabbed card.
  const planGroupStages = visibleStages.filter((s) => isPlanStage(s.stage));
  // Collect plan artifacts dynamically (plan, plan_1, plan_2, ...).
  const planArtifactMap: Record<string, string> = {};
  for (const [key, value] of Object.entries(artifacts)) {
    if (key === "plan" || /^plan_\d+$/.test(key)) {
      planArtifactMap[key] = value;
    }
  }
  let planGroupRendered = false;

  // Group all review stages for the tabbed card.
  const reviewGroupStages = visibleStages.filter((s) => isReviewStage(s.stage));
  const reviewArtifactMap: Record<string, string> = {};
  for (const [key, value] of Object.entries(artifacts)) {
    if (key === "review" || /^review_\d+$/.test(key)) {
      reviewArtifactMap[key] = value;
    }
  }
  let reviewGroupRendered = false;

  // Build parallel terminal tabs when multiple plan stages exist.
  const planTerminalTabs = planGroupStages.length > 1
    ? planGroupStages.map((s, i) => ({
        label: `Plan ${i + 1}`,
        lines: (stageLogs[s.stage] ?? []).slice(-160),
        totalLines: stageLogs[s.stage]?.length ?? 0,
      }))
    : undefined;
  // Show parallel tabs when any plan stage is running or was the most recent.
  const showPlanTerminalTabs = planTerminalTabs && (
    planGroupStages.some((s) => s.status === "running") ||
    (run.currentStage != null && isPlanStage(run.currentStage))
  );

  // Build parallel terminal tabs when multiple review stages exist.
  const reviewTerminalTabs = reviewGroupStages.length > 1
    ? reviewGroupStages.map((s, i) => ({
        label: `Review ${i + 1}`,
        lines: (stageLogs[s.stage] ?? []).slice(-160),
        totalLines: stageLogs[s.stage]?.length ?? 0,
      }))
    : undefined;
  const showReviewTerminalTabs = reviewTerminalTabs && (
    reviewGroupStages.some((s) => s.status === "running") ||
    (run.currentStage != null && isReviewStage(run.currentStage))
  );

  const headerTitle = run.prompt.length > 60 ? `${run.prompt.slice(0, 60)}...` : run.prompt;
  const isPaused = run.status === "paused";
  const iterationText = `Iteration ${Math.max(1, run.currentIteration)}/${Math.max(1, run.maxIterations)}`;

  return (
    <div className="flex h-full min-h-0 flex-col bg-[#0f0f14]">
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
        <h2 className="text-sm font-medium text-[#e4e4ed] truncate">{headerTitle}</h2>
        <span
          className={`ml-auto rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider ${statusClasses.badge}`}
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
          {visibleStages.map((stage, idx) => {
            // Group plan/plan_2/plan_3 into a single tabbed card.
            if (isPlanStage(stage.stage)) {
              if (planGroupRendered) return null;
              planGroupRendered = true;
              const planningActive = isActive(run.status) && isPlanStage(run.currentStage as PipelineStage);
              return (
                <div key="plan-group" className="flex flex-col gap-2">
                  <TabbedPlanCard
                    planStages={planGroupStages}
                    planArtifacts={planArtifactMap}
                    runPrompt={run.prompt}
                    enhancedPromptInput={enhancedPromptInput}
                    settings={settings}
                    startedAt={planningActive ? run.stageStartedAt : undefined}
                    runStatus={run.status}
                  />
                  {(planningActive || planGroupStages.some((s) => s.status === "running")) && (
                    <PlannerProgressRow
                      stages={planGroupStages}
                      settings={settings}
                      isPaused={isPaused}
                    />
                  )}
                </div>
              );
            }

            // Group code_reviewer/code_reviewer2/code_reviewer3 into a single tabbed card.
            if (isReviewStage(stage.stage)) {
              if (reviewGroupRendered) return null;
              reviewGroupRendered = true;
              const reviewingActive = isActive(run.status) && isReviewStage(run.currentStage as PipelineStage);
              return (
                <div key="review-group" className="flex flex-col gap-2">
                  <TabbedReviewCard
                    reviewStages={reviewGroupStages}
                    reviewArtifacts={reviewArtifactMap}
                    runPrompt={run.prompt}
                    enhancedPromptInput={enhancedPromptInput}
                    settings={settings}
                    startedAt={reviewingActive ? run.stageStartedAt : undefined}
                    runStatus={run.status}
                  />
                  {(reviewingActive || reviewGroupStages.some((s) => s.status === "running")) && (
                    <ReviewerProgressRow
                      stages={reviewGroupStages}
                      settings={settings}
                      isPaused={isPaused}
                    />
                  )}
                </div>
              );
            }

            return (
              <div key={`${stage.stage}-${idx}`} className="flex flex-col gap-2">
                {stage.stage === "judge" && stage.status === "completed" ? (
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
                  <RichStageCard
                    stage={stage}
                    runPrompt={run.prompt}
                    enhancedPromptInput={enhancedPromptInput}
                    promptEnhanceOutput={(enhancedPrompt ?? stage.output).trim()}
                    planOutput={resolvePlanText(planArtifact, stage.output)}
                    planInputForAudit={planInputForAudit}
                    auditedPlanOutput={resolveAuditedPlanText(planAuditArtifact, stage.output)}
                    settings={settings}
                    startedAt={
                      run.status === "running" && run.currentStage === stage.stage && stage.status === "running"
                        ? run.stageStartedAt
                        : undefined
                    }
                    showPlanCard={false}
                    showPlanAuditCard={idx === latestCompletedPlanAuditIndex}
                  />
                )}
              </div>
            );
          })}
          {isActive(run.status) && run.currentStage && !isPlanStage(run.currentStage) && !isReviewStage(run.currentStage) && (
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
              judgeReasoning={artifacts["judge"]}
            />
          )}
          {otherArtifacts.length > 0 && (
            <div className="flex flex-col gap-2">
              {otherArtifacts.map(([kind, content]) => (
                <div key={kind} className="rounded border border-[#2e2e48] bg-[#14141e] p-3">
                  <span className="text-[10px] font-semibold uppercase tracking-wider text-[#9898b0]">
                    {kind}
                  </span>
                  <pre className="mt-2 overflow-x-auto text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words">
                    {content}
                  </pre>
                </div>
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
            onTerminalScroll={terminal.onTerminalScroll}
            parallelTabs={showPlanTerminalTabs ? planTerminalTabs : showReviewTerminalTabs ? reviewTerminalTabs : undefined}
          />
        )}
        {(isActive(run.status) || isPaused) && (
          <PipelineControlBar
            statusLabel={statusLabel}
            statusClassName={statusClasses.text}
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
