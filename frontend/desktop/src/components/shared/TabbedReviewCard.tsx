import { useMemo, useState, useEffect } from "react";
import type { ReactNode } from "react";
import type { StageResult, AppSettings, PipelineStage } from "../../types";
import { formatDuration, normaliseDisplayText, truncateWords } from "../../utils/formatters";
import { stageModelLabel } from "../../utils/stageModelLabels";

/** Stages considered part of the parallel review group. */
const REVIEW_STAGES = new Set<PipelineStage>(["code_reviewer", "code_reviewer2", "code_reviewer3"]);

export function isReviewStage(stage: PipelineStage): boolean {
  return REVIEW_STAGES.has(stage);
}

interface ReviewTab {
  label: string;
  stage: PipelineStage;
  modelLabel: string;
  status: StageResult["status"];
  durationMs: number;
  error?: string;
}

interface TabbedReviewCardProps {
  /** All review stage results (code_reviewer, code_reviewer2, code_reviewer3) for this iteration. */
  reviewStages: StageResult[];
  /** Resolved review artifacts keyed by review_1, review_2, review_3 or review. */
  reviewArtifacts: Record<string, string>;
  /** Original user prompt. */
  runPrompt: string;
  /** Enhanced prompt (or original if none). */
  enhancedPromptInput: string;
  settings: AppSettings | null;
  /** Absolute timestamp when the currently running stage started. */
  startedAt?: number;
}

type ContentTab = "input" | "output";

const REVIEW_LABEL_MAP: Record<string, string> = {
  code_reviewer: "Review 1",
  code_reviewer2: "Review 2",
  code_reviewer3: "Review 3",
};

/** A single review card that groups 1-3 parallel reviewers with tabbed output. */
export function TabbedReviewCard({
  reviewStages,
  reviewArtifacts,
  runPrompt,
  enhancedPromptInput,
  settings,
  startedAt,
}: TabbedReviewCardProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [contentTab, setContentTab] = useState<ContentTab>("output");
  const [activeReviewIdx, setActiveReviewIdx] = useState(0);
  const [, tick] = useState(0);

  const hasMultipleReviewers = reviewStages.length > 1;

  const reviewTabs: ReviewTab[] = useMemo(
    () =>
      reviewStages.map((s) => ({
        label: REVIEW_LABEL_MAP[s.stage] ?? s.stage,
        stage: s.stage,
        modelLabel: stageModelLabel(s.stage, settings) ?? "",
        status: s.status,
        durationMs: s.durationMs,
        error: s.error,
      })),
    [reviewStages, settings],
  );

  // Resolve artifact per review tab: review_1/review_2/review_3, then "review", then stage output.
  const resolvedOutputs = useMemo(() => {
    if (reviewStages.length === 1) {
      return [reviewArtifacts["review"] ?? reviewStages[0].output ?? ""];
    }
    return reviewStages.map((s, i) => {
      const key = `review_${i + 1}`;
      return reviewArtifacts[key] ?? s.output ?? "";
    });
  }, [reviewStages, reviewArtifacts]);

  // Live timer tick for running stages.
  const anyRunning = reviewStages.some((s) => s.status === "running");
  useEffect(() => {
    if (!anyRunning) return;
    const interval = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(interval);
  }, [anyRunning]);

  // Aggregate status.
  const allCompleted = reviewStages.every((s) => s.status === "completed");
  const anyFailed = reviewStages.some((s) => s.status === "failed");
  const allSkipped = reviewStages.every((s) => s.status === "skipped");
  const totalDuration = reviewStages.reduce((sum, s) => Math.max(sum, s.durationMs), 0);

  const effectiveDuration =
    anyRunning && startedAt != null ? Math.max(totalDuration, Date.now() - startedAt) : totalDuration;

  const statusBadge = allCompleted
    ? { text: "Completed", cls: "text-[#22c55e] bg-[#22c55e]/10" }
    : anyFailed
      ? { text: "Failed", cls: "text-[#ef4444] bg-[#ef4444]/10" }
      : allSkipped
        ? { text: "Skipped", cls: "text-[#9898b0] bg-[#9898b0]/10" }
        : anyRunning
          ? { text: "Running", cls: "text-[#40c4ff] bg-[#40c4ff]/10" }
          : { text: "Pending", cls: "text-[#9898b0] bg-[#9898b0]/10" };

  const truncatedInputs = useMemo(
    () =>
      [
        { label: "Original Prompt", content: runPrompt },
        { label: "Enhanced Prompt", content: enhancedPromptInput },
      ]
        .map((s) => ({ ...s, preview: truncateWords(s.content, 20) }))
        .filter((s) => s.preview.length > 0),
    [runPrompt, enhancedPromptInput],
  );

  const activeReview = reviewTabs[activeReviewIdx];
  const activeOutput = resolvedOutputs[activeReviewIdx] ?? "";

  return (
    <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden">
      {/* Header */}
      <button
        type="button"
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-[#1a1a2a] transition-colors"
      >
        <svg
          className={`h-3 w-3 text-[#9898b0] transition-transform ${open ? "rotate-90" : ""}`}
          viewBox="0 0 24 24"
          fill="currentColor"
        >
          <path d="M8 5v14l11-7z" />
        </svg>
        <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed] bg-[#ffb432]/25">
          Code Review
        </span>
        {hasMultipleReviewers && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {reviewStages.length} reviewers
          </span>
        )}
        {!hasMultipleReviewers && activeReview?.modelLabel && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {activeReview.modelLabel}
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

      {/* Expanded body */}
      {open && (
        <div className="px-3 pb-3">
          {/* Content tabs: Input / Output */}
          <div className="mb-2 flex gap-1">
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setContentTab("input"); }}
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
              onClick={(e) => { e.stopPropagation(); setContentTab("output"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                contentTab === "output"
                  ? "bg-[#22c55e]/20 text-[#4ade80]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Output
            </button>
          </div>

          {/* Input tab */}
          {contentTab === "input" && (
            <div className="flex flex-col gap-3">
              {truncatedInputs.map((section) => (
                <div key={section.label}>
                  <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                    {section.label}
                  </span>
                  <div className="rounded bg-[#0f0f14] px-3 py-2 text-xs text-[#c8c8d8] whitespace-pre-wrap leading-relaxed">
                    {section.preview}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Output tab — with sub-tabs per reviewer when multiple */}
          {contentTab === "output" && (
            <div>
              {hasMultipleReviewers && (
                <ReviewSubTabs
                  tabs={reviewTabs}
                  activeIdx={activeReviewIdx}
                  onSelect={setActiveReviewIdx}
                />
              )}
              <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                {activeReview?.label ?? "Review"}
              </span>
              {activeReview?.error && (
                <p className="mb-1 text-xs text-[#ef4444]">{activeReview.error}</p>
              )}
              <pre className="rounded border border-orange-400/20 bg-orange-400/5 px-3 py-2 text-xs text-[#e4e4ed] whitespace-pre-wrap leading-relaxed break-words">
                {normaliseDisplayText(activeOutput) || "No review output generated."}
              </pre>
            </div>
          )}
        </div>
      )}
    </article>
  );
}

/** Sub-tab row for switching between parallel reviewers. */
function ReviewSubTabs({
  tabs,
  activeIdx,
  onSelect,
}: {
  tabs: ReviewTab[];
  activeIdx: number;
  onSelect: (idx: number) => void;
}): ReactNode {
  return (
    <div className="mb-2 flex gap-1">
      {tabs.map((tab, idx) => {
        const isActive = idx === activeIdx;
        const statusDot =
          tab.status === "completed"
            ? "bg-[#22c55e]"
            : tab.status === "failed"
              ? "bg-[#ef4444]"
              : tab.status === "running"
                ? "bg-[#ffb432] animate-pulse"
                : "bg-[#9898b0]";

        return (
          <button
            key={tab.stage}
            type="button"
            onClick={(e) => { e.stopPropagation(); onSelect(idx); }}
            className={`flex items-center gap-1.5 rounded px-2 py-1 text-[10px] font-medium transition-colors ${
              isActive
                ? "bg-[#ffb432]/15 text-[#ffb432]"
                : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
            }`}
          >
            <span className={`inline-block h-1.5 w-1.5 rounded-full ${statusDot}`} />
            {tab.label}
            {tab.modelLabel && (
              <span className="text-[8px] opacity-60">{tab.modelLabel}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}
