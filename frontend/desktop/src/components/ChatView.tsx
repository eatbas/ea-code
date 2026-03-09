import type { ReactNode } from "react";
import { useEffect, useRef } from "react";
import type { PipelineRun, RunOptions, CliHealth, AppSettings } from "../types";
import { isActive, isTerminal, statusInfo } from "../utils/statusHelpers";
import { StageCard } from "./shared/StageCard";
import { ThinkingIndicator } from "./shared/ThinkingIndicator";
import { ResultCard } from "./shared/ResultCard";
import { ArtifactCard } from "./shared/ArtifactCard";
import { PromptCard } from "./shared/PromptCard";
import { PromptInputBar } from "./shared/PromptInputBar";

/** Artefact kinds consumed by ResultCard — excluded from the generic list. */
const RESULT_ARTIFACT_KINDS = new Set(["result", "executive_summary", "judge"]);

interface ChatViewProps {
  run: PipelineRun;
  stageLogs: Record<string, string[]>;
  artifacts: Record<string, string>;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onMissingAgentSetup: () => void;
  onCancel: () => void;
  onBackToHome: () => void;
  onContinue: (options: RunOptions) => void;
  onViewSession: () => void;
}

/** Chat-style running view with stage timeline, result card, and follow-up input. */
export function ChatView({
  run,
  stageLogs,
  artifacts,
  cliHealth,
  settings,
  onMissingAgentSetup,
  onCancel,
  onBackToHome,
  onContinue,
  onViewSession,
}: ChatViewProps): ReactNode {
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new content arrives
  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [run.iterations.length, Object.keys(artifacts).length, Object.keys(stageLogs).length]);

  const { label: statusLabel, colour: statusColour } = statusInfo(run.status);

  // Flatten all completed stages for timeline display
  const allStages = run.iterations.flatMap((iter) => iter.stages);

  // Non-result artifacts for the generic list
  const enhancedPrompt = artifacts["enhanced_prompt"];
  const otherArtifacts = Object.entries(artifacts).filter(
    ([kind]) => !RESULT_ARTIFACT_KINDS.has(kind) && kind !== "workspace_context" && kind !== "enhanced_prompt",
  );

  return (
    <div className="flex h-full flex-col bg-[#0f0f14]">
      {/* Scrollable timeline area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto px-6 pt-6 pb-4">
        <div className="mx-auto max-w-2xl flex flex-col gap-3">
          {/* User prompt — right-aligned bubble */}
          <div className="flex justify-end">
            <div className="max-w-[80%] rounded-2xl rounded-br-md bg-[#2a2a3e] px-4 py-3 text-sm text-[#e4e4ed] whitespace-pre-wrap">
              {run.prompt}
            </div>
          </div>

          {/* Stage timeline cards */}
          {allStages.map((stage, idx) => (
            <StageCard key={`${stage.stage}-${idx}`} stage={stage} logs={stageLogs[stage.stage]} />
          ))}

          {/* Original vs Enhanced prompt comparison */}
          {enhancedPrompt && (
            <PromptCard originalPrompt={run.prompt} enhancedPrompt={enhancedPrompt} />
          )}

          {/* Thinking indicator for currently running stage */}
          {isActive(run.status) && run.currentStage && (
            <ThinkingIndicator stage={run.currentStage} />
          )}

          {/* Result card when pipeline reaches terminal state */}
          {isTerminal(run.status) && (
            <ResultCard run={run} artifacts={artifacts} />
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
        {isActive(run.status) && (
          <div className="flex w-full items-center gap-2 rounded-xl border border-[#2e2e48] bg-[#1a1a24] px-4 py-3">
            <div className="flex items-center gap-2 flex-1">
              <svg className="animate-spin h-3.5 w-3.5" style={{ color: statusColour }} xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
              </svg>
              <span className="text-sm text-[#9898b0]">{statusLabel}...</span>
            </div>
            <button
              onClick={onCancel}
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
          <>
            <PromptInputBar
              placeholder="Continue this session..."
              cliHealth={cliHealth}
              settings={settings}
              onMissingAgentSetup={onMissingAgentSetup}
              onSubmit={onContinue}
            />
            <div className="flex items-center justify-between px-1 text-xs text-[#9898b0]">
              <button
                onClick={onViewSession}
                className="hover:text-[#e4e4ed] transition-colors"
              >
                View session
              </button>
              <button
                onClick={onBackToHome}
                className="hover:text-[#e4e4ed] transition-colors"
              >
                New conversation
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
