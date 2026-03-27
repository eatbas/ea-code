import type { ReactNode } from "react";
import { Checkmark } from "../shared/Checkmark";

interface ModelOption {
  value: string;
  label: string;
}

interface ModelCheckboxListProps {
  /** Available model options for this provider. */
  modelOptions: ModelOption[];
  /** Set of currently enabled model identifiers. */
  enabledModels: Set<string>;
  /** Whether checkbox interactions should be disabled. */
  disabled: boolean;
  /** Called when a single model is toggled. */
  onToggleModel: (value: string) => void;
  /** Called to select or deselect all models. */
  onToggleAll: (selectAll: boolean) => void;
}

/** Checkbox list for selecting which models are enabled for a CLI provider. */
export function ModelCheckboxList({
  modelOptions,
  enabledModels,
  disabled,
  onToggleModel,
  onToggleAll,
}: ModelCheckboxListProps): ReactNode {
  const allSelected = modelOptions.every((opt) => enabledModels.has(opt.value));

  return (
    <div className="mt-4">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-[10px] font-medium uppercase tracking-wider text-fg-faint">
          Models
        </p>
        <button
          type="button"
          onClick={() => onToggleAll(!allSelected)}
          disabled={disabled}
          className="flex items-center gap-1.5 text-[10px] font-medium text-fg-faint transition-colors hover:text-fg disabled:cursor-not-allowed disabled:opacity-50"
        >
          <span
            className={`flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border ${
              allSelected
                ? "border-fg bg-fg"
                : "border-edge-strong bg-transparent"
            }`}
          >
            {allSelected && (
              <Checkmark size="sm" className="text-surface" />
            )}
          </span>
          Select all
        </button>
      </div>
      <div className="flex flex-col gap-1.5">
        {modelOptions.map((opt) => {
          const isChecked = enabledModels.has(opt.value);
          return (
            <button
              key={opt.value}
              type="button"
              onClick={() => onToggleModel(opt.value)}
              disabled={disabled}
              className={`flex items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
                isChecked
                  ? "bg-elevated text-fg"
                  : disabled
                    ? "bg-surface text-fg-faint"
                    : "bg-surface text-fg-muted hover:bg-elevated hover:text-fg"
              } disabled:cursor-not-allowed disabled:opacity-50`}
            >
              <span
                className={`flex h-4 w-4 shrink-0 items-center justify-center rounded border ${
                  isChecked
                    ? "border-fg bg-fg"
                    : "border-edge-strong bg-transparent"
                }`}
              >
                {isChecked && (
                  <Checkmark size="md" className="text-surface" />
                )}
              </span>
              {opt.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
