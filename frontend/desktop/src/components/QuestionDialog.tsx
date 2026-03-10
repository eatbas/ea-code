import type { ReactNode } from "react";
import { useState } from "react";
import type { PipelineQuestionEvent, PipelineAnswer } from "../types";
import { STAGE_LABELS } from "./shared/constants";

interface QuestionDialogProps {
  question: PipelineQuestionEvent;
  onAnswer: (answer: PipelineAnswer) => void;
}

/** Modal overlay shown when the pipeline pauses for user input. */
export function QuestionDialog({ question, onAnswer }: QuestionDialogProps): ReactNode {
  const [response, setResponse] = useState<string>("");

  function handleSubmit(): void {
    onAnswer({
      runId: question.runId,
      questionId: question.questionId,
      answer: response,
      skipped: false,
    });
  }

  function handleSkip(): void {
    onAnswer({
      runId: question.runId,
      questionId: question.questionId,
      answer: "",
      skipped: true,
    });
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>): void {
    // Cmd/Ctrl + Enter to submit
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  }

  const stageLabel = STAGE_LABELS[question.stage] ?? question.stage;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="w-[640px] max-h-[80vh] flex flex-col rounded-lg border border-[#2e2e48] bg-[#1a1a24] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-[#2e2e48] px-4 py-3">
          <div className="flex items-center gap-2">
            <span className="rounded bg-[#f59e0b] px-2 py-0.5 text-xs font-medium text-white">
              AWAITING INPUT
            </span>
            <span className="text-sm font-medium text-[#e4e4ed]">
              {stageLabel} Stage
            </span>
          </div>
          <span className="text-xs text-[#9898b0]">
            Iteration {question.iteration}
          </span>
        </div>

        {/* Agent output preview */}
        <div className="max-h-48 overflow-y-auto border-b border-[#2e2e48] bg-[#0f0f14] p-3">
          <pre className="font-mono text-xs text-[#e4e4ed] whitespace-pre-wrap break-words">
            {question.agentOutput}
          </pre>
        </div>

        {/* Question text */}
        <div className="px-4 py-3">
          <p className="text-sm text-[#e4e4ed]">{question.questionText}</p>
        </div>

        {/* Answer textarea */}
        <div className="px-4 pb-3">
          <textarea
            value={response}
            onChange={(e) => setResponse(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Type your guidance or answer... (Cmd+Enter to submit)"
            rows={4}
            className="w-full resize-y rounded border border-[#2e2e48] bg-[#0f0f14] p-3 text-sm text-[#e4e4ed] placeholder-[#9898b0] focus:border-[#6366f1] focus:outline-none"
            autoFocus
          />
        </div>

        {/* Action buttons */}
        <div className="flex items-center justify-end gap-2 border-t border-[#2e2e48] px-4 py-3">
          {question.optional && (
            <button
              type="button"
              onClick={handleSkip}
              className="rounded border border-[#2e2e48] bg-[#24243a] px-4 py-2 text-sm text-[#9898b0] hover:bg-[#2e2e48] hover:text-[#e4e4ed] transition-colors"
            >
              Skip
            </button>
          )}
          <button
            type="button"
            onClick={handleSubmit}
            className="rounded bg-[#6366f1] px-4 py-2 text-sm font-medium text-white hover:bg-[#818cf8] transition-colors"
          >
            Submit
          </button>
        </div>
      </div>
    </div>
  );
}
