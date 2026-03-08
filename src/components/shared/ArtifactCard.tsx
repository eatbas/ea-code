import type { ReactNode } from "react";
import { ARTIFACT_LABELS } from "./constants";

interface ArtifactCardProps {
  kind: string;
  content: string;
  defaultOpen?: boolean;
}

/** Collapsible card for a pipeline artefact (diff, plan, review, etc.). */
export function ArtifactCard({ kind, content, defaultOpen }: ArtifactCardProps): ReactNode {
  const label = ARTIFACT_LABELS[kind] ?? kind;
  const isDiff = kind === "diff" || kind.startsWith("diff_");

  return (
    <details open={defaultOpen} className="rounded-lg border border-[#2e2e48] bg-[#14141e]">
      <summary className="cursor-pointer px-3 py-2 text-[10px] font-medium text-[#9898b0] hover:text-[#e4e4ed] transition-colors">
        {label}
      </summary>
      <div className="border-t border-[#2e2e48] px-3 py-2">
        {isDiff ? (
          <pre className="max-h-64 overflow-auto text-[11px] leading-relaxed whitespace-pre-wrap break-words font-mono">
            {content.split("\n").map((line, i) => {
              const colour = line.startsWith("+") ? "#22c55e"
                : line.startsWith("-") ? "#ef4444"
                : "#e4e4ed";
              return (
                <div key={i} style={{ color: colour }}>{line}</div>
              );
            })}
          </pre>
        ) : (
          <pre className="max-h-64 overflow-auto text-[11px] text-[#e4e4ed] whitespace-pre-wrap break-words font-mono">
            {content}
          </pre>
        )}
      </div>
    </details>
  );
}
