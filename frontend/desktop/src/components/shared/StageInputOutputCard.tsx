import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import { formatDuration, truncateWords } from "../../utils/formatters";

interface InputSection {
  label: string;
  content: string;
}

interface StageInputOutputCardProps {
  title: string;
  inputSections: InputSection[];
  outputLabel: string;
  outputContent: string;
  modelLabel?: string;
  durationMs?: number;
  inputPreviewWords?: number;
  badgeClassName?: string;
  outputClassName?: string;
}

type StageTab = "input" | "output";

/** Collapsible stage card with Input/Output tabs for planning and auditing stages. */
export function StageInputOutputCard({
  title,
  inputSections,
  outputLabel,
  outputContent,
  modelLabel,
  durationMs,
  inputPreviewWords = 20,
  badgeClassName = "bg-sky-400/25",
  outputClassName = "border border-sky-400/20 bg-sky-400/5 text-[#e4e4ed]",
}: StageInputOutputCardProps): ReactNode {
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<StageTab>("output");

  const truncatedInputs = useMemo(
    () => inputSections
      .map((section) => ({
        ...section,
        preview: truncateWords(section.content, inputPreviewWords),
      }))
      .filter((section) => section.preview.length > 0),
    [inputPreviewWords, inputSections],
  );

  return (
    <article className="rounded-lg border border-[#2e2e48] bg-[#14141e] overflow-hidden">
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
        <span className={`rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-widest text-[#e4e4ed] ${badgeClassName}`}>
          {title}
        </span>
        {modelLabel && (
          <span className="rounded bg-[#2e2e48] px-1.5 py-0.5 text-[9px] font-medium text-[#c8c8d8]">
            {modelLabel}
          </span>
        )}

        <div className="ml-auto flex items-center gap-2 text-[10px]">
          {durationMs != null && durationMs > 0 && (
            <span className="text-[#9898b0] opacity-80">{formatDuration(durationMs)}</span>
          )}
          <span className="rounded px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider text-[#22c55e] bg-[#22c55e]/10">
            Completed
          </span>
        </div>
      </button>

      {open && (
        <div className="px-3 pb-3">
          <div className="mb-2 flex gap-1">
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setTab("input"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                tab === "input"
                  ? "bg-[#9898b0]/20 text-[#e4e4ed]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Input
            </button>
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); setTab("output"); }}
              className={`rounded px-2.5 py-1 text-[10px] font-medium uppercase tracking-wider transition-colors ${
                tab === "output"
                  ? "bg-[#22c55e]/20 text-[#4ade80]"
                  : "text-[#9898b0] hover:text-[#c8c8d8] hover:bg-[#9898b0]/10"
              }`}
            >
              Output
            </button>
          </div>

          {tab === "input" && (
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

          {tab === "output" && (
            <div>
              <span className="mb-1 block text-[10px] font-medium uppercase tracking-wider text-[#9898b0]">
                {outputLabel}
              </span>
              <pre className={`rounded px-3 py-2 text-xs whitespace-pre-wrap leading-relaxed break-words ${outputClassName}`}>
                {outputContent}
              </pre>
            </div>
          )}
        </div>
      )}
    </article>
  );
}
