import type { ReactNode } from "react";
import type { AgentBackend, AppSettings, CliHealth } from "../../types";
import { CascadingSelect } from "./CascadingSelect";

export interface InlineStageSlotProps {
  label: string;
  /** Direct backend value (for array-based extra slots). */
  backend: AgentBackend | null;
  /** Direct model value (for array-based extra slots). */
  model: string | null;
  draft: AppSettings;
  cliHealth: CliHealth | null;
  cliHealthChecking: boolean;
  /** Called when the user selects a new backend + model. */
  onChange: (backend: AgentBackend | null, model: string | null) => void;
  onRemove: () => void;
}

/** Additional stage selector rendered inline within a parent card. */
export function InlineStageSlot({
  label,
  backend,
  model,
  draft,
  cliHealth,
  cliHealthChecking,
  onChange,
  onRemove,
}: InlineStageSlotProps): ReactNode {
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
        backend={backend}
        model={model}
        settings={draft}
        optional={true}
        cliHealth={cliHealth}
        cliHealthChecking={cliHealthChecking}
        onChange={onChange}
      />
    </div>
  );
}
