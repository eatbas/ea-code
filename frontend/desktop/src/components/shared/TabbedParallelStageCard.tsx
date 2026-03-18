import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { AppSettings, PipelineStage, StageResult } from "../../types";
import { formatDuration, normaliseDisplayText, truncateWords } from "../../utils/formatters";
import { stageModelLabel } from "../../utils/stageModelLabels";

interface ParallelStageTab {
  label: string;
  stage: PipelineStage;
  modelLabel: string;
  status: StageResult["status"];
  durationMs: number;
  error?: string;
}

export interface ParallelStageCardConfig {
  heading: string;
  headingBadgeClassName: string;
  countNoun: string;
  outputLabel: string;
  outputEmptyText: string;
  stageLabelMap: Record<string, string>;
  singleArtifactKey: string;
  artifactKeyPrefix: string;
  activeSubTabBackgroundClassName: string;
  activeSubTabTextClassName: string;
  activeSubTabDotClassName: string;
  outputBorderClassName: string;
  outputBackgroundClassName: string;
}

interface TabbedParallelStageCardProps {
  stages: StageResult[];
  artifacts: Record<string, string>;
  runPrompt: string;
  enhancedPromptInput: string;
  settings: AppSettings | null;
  startedAt?: number;
  config: ParallelStageCardConfig;
}

type ContentTab = "input" | "output";

const INACTIVE_SUBTAB_CLASS = "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10";
const INACTIVE_SUBTAB_DOT_CLASS = "bg-[#9898b0]";
const STATUS_BADGES = {
  completed: { text: "Completed", cls: "text-[#22c55e] bg-[#22c55e]/10" },
  failed: { text: "Failed", cls: "text-[#ef4444] bg-[#ef4444]/10" },
  skipped: { text: "Skipped", cls: "text-[#9898b0] bg-[#9898b0]/10" },
  running: { text: "Running", cls: "text-[#40c4ff] bg-[#40c4ff]/10" },
  pending: { text: "Pending", cls: "text-[#9898b0] bg-[#9898b0]/10" },
} as const;

function resolveStatusBadge(stages: StageResult[]) {
  const allCompleted = stages.every((stage) => stage.status === "completed");
  const anyFailed = stages.some((stage) => stage.status === "failed");
  const allSkipped = stages.every((stage) => stage.status === "skipped");
  const anyRunning = stages.some((stage) => stage.status === "running");

  if (allCompleted) return STATUS_BADGES.completed;
  if (anyFailed) return STATUS_BADGES.failed;
  if (allSkipped) return STATUS_BADGES.skipped;
  if (anyRunning) return STATUS_BADGES.running;
  return STATUS_BADGES.pending;
}

function resolveOutputs(
  stages: StageResult[],
  artifacts: Record<string, string>,
  singleArtifactKey: string,
  artifactKeyPrefix: string,
): string[] {
  if (stages.length === 1) {
    return [artifacts[singleArtifactKey] ?? stages[0].output ?? ""];
  }

  return stages.map((stage, index) => {
    const artifactKey = `${artifactKeyPrefix}_${index + 1}`;
    return artifacts[artifactKey] ?? stage.output ?? "";
  });
}

function buildTabs(stages: StageResult[], settings: AppSettings | null, labelMap: Record<string, string>): ParallelStageTab[] {
  return stages.map((stage) => ({
    label: labelMap[stage.stage] ?? stage.stage,
    stage: stage.stage,
    modelLabel: stageModelLabel(stage.stage, settings) ?? "",
    status: stage.status,
    durationMs: stage.durationMs,
    error: stage.error,
  }));
}

function getStatusDotClass(status: StageResult["status"], activeDotClassName: string): string {
  if (status === "completed") return "bg-[#22c55e]";
  if (status === "failed") return "bg-[#ef4444]";
  if (status === "running") return `${activeDotClassName} animate-pulse`;
  return INACTIVE_SUBTAB_DOT_CLASS;
}

function ParallelStageSubTabs({
  tabs,
  activeIdx,
  onSelect,
  activeSubTabBackgroundClassName,
  activeSubTabTextClassName,
  activeSubTabDotClassName,
}: {
  tabs: ParallelStageTab[];
  activeIdx: number;
  onSelect: (idx: number) => void;
  activeSubTabBackgroundClassName: string;
  activeSubTabTextClassName: string;
  activeSubTabDotClassName: string;
}): ReactNode {
  return (
    <div className="mb-2 flex gap-1">
      {tabs.map((tab, idx) => {
        const isActive = idx === activeIdx;
        const statusDot = getStatusDotClass(tab.status, activeSubTabDotClassName);

        return (
          <button
            key={tab.stage}
            type="button"
            onClick={(event) => { event.stopPropagation(); onSelect(idx); }}
            className={`flex items-center gap-1.5 rounded px-2 py-1 text-[10px] font-medium transition-colors ${
              isActive
                ? `${activeSubTabBackgroundClassName} ${activeSubTabTextClassName}`
                : INACTIVE_SUBTAB_CLASS
            }`}
          >
            <span className={`inline-block h-1.5 w-1.5 rounded-full ${statusDot}`} />
            {tab.label}
            {tab.modelLabel && <span className="text-[8px] opacity-60">{tab.modelLabel}</span>}
          </button>
        );
      })}
    </div>
  );
}

/** Generic card for parallel planning/review stages with tabbed outputs. */
export function TabbedParallelStageCard({
  stages,
  artifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
  config,
}: TabbedParallelStageCardProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [contentTab, setContentTab] = useState<ContentTab>("output");
  const [activeTabIdx, setActiveTabIdx] = useState(0);
  const [, tick] = useState(0);

  const hasMultipleStages = stages.length > 1;

  const tabs = useMemo(
    () => buildTabs(stages, settings, config.stageLabelMap),
    [config.stageLabelMap, settings, stages],
  );
  const resolvedOutputs = useMemo(
    () => resolveOutputs(stages, artifacts, config.singleArtifactKey, config.artifactKeyPrefix),
    [artifacts, config.artifactKeyPrefix, config.singleArtifactKey, stages],
  );

  const anyRunning = stages.some((stage) => stage.status === "running");
  useEffect(() => {
    if (!anyRunning) return;
    const interval = window.setInterval(() => tick((value) => value + 1), 1000);
    return () => window.clearInterval(interval);
  }, [anyRunning]);

  const totalDuration = stages.reduce((sum, stage) => Math.max(sum, stage.durationMs), 0);
  const effectiveDuration =
    anyRunning && startedAt != null ? Math.max(totalDuration, Date.now() - startedAt) : totalDuration;
  const statusBadge = resolveStatusBadge(stages);

  const truncatedInputs = useMemo(
    () =>
      [
        { label: "Original Prompt", content: runPrompt },
        { label: "Enhanced Prompt", content: enhancedPromptInput },
      ]
        .map((section) => ({ ...section, preview: truncateWords(section.content, 20) }))
        .filter((section) => section.preview.length > 0),
    [enhancedPromptInput, runPrompt],
  );

  const activeTab = tabs[activeTabIdx];
  const activeOutput = resolvedOutputs[activeTabIdx] ?? "";

  return (
    <article className="overflow-hidden rounded-lg border border-[#2e2e48] bg-[#14141e]">
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left transition-colors hover:bg-[#1a1a2a]"
      >
        <svg
          className={`h-3 w-3 text-[#9898b0] transition-transform ${open ? "rotate-90" : ""}`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed] ${config.headingBadgeClassName}`}>
          {config.heading}
        </span>
        {hasMultipleStages && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {stages.length} {config.countNoun}
          </span>
        )}
        {!hasMultipleStages && activeTab?.modelLabel && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {activeTab.modelLabel}
          </span>
        )}

        <div className="ml-auto flex items-center gap-2 text-[10px]">
          {effectiveDuration > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(effectiveDuration)}</span>
          )}
          <span className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider ${statusBadge.cls}`}>
            {statusBadge.text}
          </span>
        </div>
      </button>

      {open && (
        <div className="px-3 pb-3">
          <div className="mb-2 flex gap-1">
            <button
              type="button"
              onClick={(event) => { event.stopPropagation(); setContentTab("input"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                contentTab === "input"
                  ? "bg-[#9898b0]/20 text-[#e4e4ed]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Input
            </button>
            <button
              type="button"
              onClick={(event) => { event.stopPropagation(); setContentTab("output"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                contentTab === "output"
                  ? "bg-[#22c55e]/20 text-[#4ade80]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Output
            </button>
          </div>

          {contentTab === "input" && (
            <div className="flex flex-col gap-3">
              {truncatedInputs.map((section) => (
                <div key={section.label}>
                  <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                    {section.label}
                  </span>
                  <div className="whitespace-pre-wrap rounded bg-[#0f0f14] px-3 py-2 text-xs leading-relaxed text-[#c8c8d8]">
                    {section.preview}
                  </div>
                </div>
              ))}
            </div>
          )}

          {contentTab === "output" && (
            <div>
              {hasMultipleStages && (
                <ParallelStageSubTabs
                  tabs={tabs}
                  activeIdx={activeTabIdx}
                  onSelect={setActiveTabIdx}
                  activeSubTabBackgroundClassName={config.activeSubTabBackgroundClassName}
                  activeSubTabTextClassName={config.activeSubTabTextClassName}
                  activeSubTabDotClassName={config.activeSubTabDotClassName}
                />
              )}
              <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                {activeTab?.label ?? config.outputLabel}
              </span>
              {activeTab?.error && <p className="mb-1 text-xs text-[#ef4444]">{activeTab.error}</p>}
              <pre className={`whitespace-pre-wrap break-words rounded border px-3 py-2 text-xs leading-relaxed text-[#e4e4ed] ${config.outputBorderClassName} ${config.outputBackgroundClassName}`}>
                {normaliseDisplayText(activeOutput) || config.outputEmptyText}
              </pre>
            </div>
          )}
        </div>
      )}
    </article>
  );
}
