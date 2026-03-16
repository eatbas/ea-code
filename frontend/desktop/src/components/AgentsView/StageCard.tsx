import type { ReactNode } from "react";
import type { AppSettings, AgentBackend, CliHealth } from "../../types";
import { CascadingSelect } from "./CascadingSelect";

/** Props for a single stage card. */
export interface StageCardProps {
  label: string;
  tag: string;
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
  optional: boolean;
  draft: AppSettings;
  cliHealth: CliHealth | null;
  cliHealthChecking: boolean;
  onUpdate: (patch: Partial<AppSettings>) => void;
  onRemove?: () => void;
}

/** A single pipeline stage agent card with optional remove button. */
export function StageCard({
  label, tag, backendKey, modelKey, optional,
  draft, cliHealth, cliHealthChecking, onUpdate, onRemove,
}: StageCardProps): ReactNode {
  const currentBackend = draft[backendKey] as AgentBackend | null;
  const currentModel = draft[modelKey] as string | null;
  const isMandatoryUnconfigured = !optional && (!currentBackend || !currentModel);

  return (
    <div className="relative rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4 flex flex-col gap-2">
      {isMandatoryUnconfigured && (
        <span aria-hidden="true" className="absolute inset-y-0 left-0 w-1.5 rounded-l-lg bg-[#dc2626]" />
      )}
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-[#9898b0]">
          {label}
          <span className="ml-1 text-[#6b6b80]">{tag}</span>
        </span>
        {onRemove && (
          <button
            onClick={onRemove}
            className="text-[#6b6b80] hover:text-[#ef4444] text-xs px-1"
            title="Remove this slot"
          >
            ✕
          </button>
        )}
      </div>
      <CascadingSelect
        backend={currentBackend}
        model={currentModel}
        settings={draft}
        optional={optional}
        cliHealth={cliHealth}
        cliHealthChecking={cliHealthChecking}
        onChange={(newBackend, newModel) => {
          onUpdate({
            [backendKey]: newBackend,
            [modelKey]: newModel ?? "",
          } as Partial<AppSettings>);
        }}
      />
    </div>
  );
}
