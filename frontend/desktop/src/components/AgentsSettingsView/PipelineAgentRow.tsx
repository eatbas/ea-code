import type { ReactNode } from "react";
import { useMemo } from "react";
import type { PipelineAgent, ProviderInfo } from "../../types";
import { PopoverSelect } from "../shared/PopoverSelect";
import { modelOptionsFromProvider, providerDisplayName } from "../shared/constants";

interface PipelineAgentRowProps {
  agent: PipelineAgent;
  providers: ProviderInfo[];
  /** Unique key used to namespace popover open state. */
  slotKey: string;
  openSelect: string | null;
  onOpenSelectChange: (key: string | null) => void;
  onChange: (agent: PipelineAgent) => void;
  /** Show remove button. */
  removable?: boolean;
  onRemove?: () => void;
}

export function PipelineAgentRow({
  agent,
  providers,
  slotKey,
  openSelect,
  onOpenSelectChange,
  onChange,
  removable = false,
  onRemove,
}: PipelineAgentRowProps): ReactNode {
  const selectedProvider = providers.find((p) => p.name === agent.provider) ?? providers[0];
  const providerValue = selectedProvider?.name ?? "";
  const modelValue = selectedProvider?.models.includes(agent.model)
    ? agent.model
    : selectedProvider?.models[0] ?? "";
  const modelOptions = modelOptionsFromProvider(selectedProvider);
  const providerOptions = useMemo(
    () => providers.map((p) => ({ value: p.name, label: providerDisplayName(p.name) })),
    [providers],
  );

  const providerKey = `${slotKey}-provider`;
  const modelKey = `${slotKey}-model`;

  return (
    <div className="flex items-center gap-2">
      <PopoverSelect
        value={providerValue}
        options={providerOptions}
        placeholder="Provider"
        direction="down"
        align="left"
        open={openSelect === providerKey}
        onOpenChange={(open) => onOpenSelectChange(open ? providerKey : null)}
        onChange={(value) => {
          const next = providers.find((p) => p.name === value);
          if (!next) return;
          onChange({ provider: next.name, model: next.models[0] ?? "" });
        }}
      />
      <span className="text-xs text-fg-faint">/</span>
      <PopoverSelect
        value={modelValue}
        options={modelOptions}
        placeholder="Model"
        direction="down"
        align="left"
        open={openSelect === modelKey}
        onOpenChange={(open) => onOpenSelectChange(open ? modelKey : null)}
        onChange={(value) => onChange({ ...agent, model: value })}
      />
      {removable && onRemove && (
        <button
          type="button"
          onClick={onRemove}
          className="rounded p-1 text-fg-faint transition-colors hover:bg-elevated hover:text-fg"
          title="Remove"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      )}
    </div>
  );
}
