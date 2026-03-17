import type { ReactNode } from "react";
import type { AgentBackend, AppSettings, CliHealth } from "../../types";
import { CascadingSelect } from "./CascadingSelect";

export interface InlineStageSlotProps {
  label: string;
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
  optional: boolean;
  draft: AppSettings;
  cliHealth: CliHealth | null;
  cliHealthChecking: boolean;
  onUpdate: (patch: Partial<AppSettings>) => void;
  onRemove: () => void;
}

/** Additional stage selector rendered inline within a parent card. */
export function InlineStageSlot({
  label,
  backendKey,
  modelKey,
  optional,
  draft,
  cliHealth,
  cliHealthChecking,
  onUpdate,
  onRemove,
}: InlineStageSlotProps): ReactNode {
  const currentBackend = draft[backendKey] as AgentBackend | null;
  const currentModel = draft[modelKey] as string | null;

  return (
    <div className="flex flex-col gap-2 border-t border-[#2e2e48] pt-3">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-[#9898b0]">
          {label}
          <span className="ml-1 text-[#6b6b80]">(optional)</span>
        </span>
        <button
          type="button"
          onClick={onRemove}
          className="px-1 text-xs text-[#6b6b80] hover:text-[#ef4444]"
          title="Remove this slot"
        >
          ✕
        </button>
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
