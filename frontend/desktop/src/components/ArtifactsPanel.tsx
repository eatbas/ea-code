import type { ReactNode } from "react";
import { useState } from "react";

interface ArtifactsPanelProps {
  artifacts: Record<string, string>;
}

/** Tab identifiers for artifact display. */
const TABS = ["plan", "plan_audit", "plan_final", "review", "judge", "executive_summary", "diff"] as const;
type ArtifactTab = (typeof TABS)[number];

/** Tab labels for display. */
const TAB_LABELS: Record<ArtifactTab, string> = {
  plan: "Plan",
  plan_audit: "Plan Audit",
  plan_final: "Final Plan",
  review: "Review",
  judge: "Judge",
  executive_summary: "Summary",
  diff: "Diff",
};

/** Tabbed panel displaying pipeline output artefacts. */
export function ArtifactsPanel({ artifacts }: ArtifactsPanelProps): ReactNode {
  const [activeTab, setActiveTab] = useState<ArtifactTab>("plan");
  const content = artifacts[activeTab];

  return (
    <div className="flex flex-col h-full">
      {/* Tab bar */}
      <div className="flex border-b border-[#2e2e48]">
        {TABS.map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-4 py-2 text-xs font-medium transition-colors ${
              activeTab === tab
                ? "text-[#6366f1] border-b-2 border-[#6366f1]"
                : "text-[#9898b0] hover:text-[#e4e4ed]"
            }`}
          >
            {TAB_LABELS[tab]}
          </button>
        ))}
      </div>

      {/* Content area */}
      <div className="flex-1 overflow-y-auto bg-[#0f0f14] p-3">
        {content ? (
          <pre className="font-mono text-xs text-[#e4e4ed] whitespace-pre-wrap break-words">
            {content}
          </pre>
        ) : (
          <span className="text-sm text-[#9898b0]">No content yet.</span>
        )}
      </div>
    </div>
  );
}
