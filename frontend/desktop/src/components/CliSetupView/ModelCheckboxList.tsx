import { useRef, useState, useEffect, type ReactNode } from "react";
import { Checkmark } from "../shared/Checkmark";
import { PopoverSelect } from "../shared/PopoverSelect";

interface ModelOption {
  value: string;
  label: string;
}

interface ThinkingOption {
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
  /** Per-model thinking / effort options keyed by model value (undefined = no thinking control). */
  thinkingOptions: Record<string, ThinkingOption[]> | undefined;
  /** Per-model thinking levels keyed by model value. */
  thinkingLevels: Record<string, string>;
  /** Short labels for the trigger button, keyed by option value. */
  thinkingTriggerLabels: Record<string, string> | undefined;
  /** Called when a single model is toggled. */
  onToggleModel: (value: string) => void;
  /** Called to select or deselect all models. */
  onToggleAll: (selectAll: boolean) => void;
  /** Called when the thinking level changes for a model. */
  onThinkingChange: (model: string, value: string) => void;
}

const THINKING_TRIGGER_CLASS =
  "flex h-8 w-[5.5rem] items-center gap-1 rounded-lg border border-edge-strong bg-input-bg px-2 text-xs font-medium text-fg shadow-[0_10px_24px_rgba(0,0,0,0.22)] transition-all hover:border-input-border-focus hover:bg-elevated disabled:cursor-not-allowed disabled:opacity-55";

const THINKING_MENU_CLASS =
  "w-max min-w-full rounded-2xl border border-edge-strong bg-panel p-1 shadow-[0_18px_40px_rgba(0,0,0,0.35)] backdrop-blur";

/** Span that shows a title tooltip only when the text is truncated. */
function TruncatedLabel({ text }: { text: string }): ReactNode {
  const ref = useRef<HTMLSpanElement>(null);
  const [isTruncated, setIsTruncated] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (el) {
      setIsTruncated(el.scrollWidth > el.clientWidth);
    }
  }, [text]);

  return (
    <span
      ref={ref}
      className="truncate"
      title={isTruncated ? text : undefined}
    >
      {text}
    </span>
  );
}

/** Checkbox list for selecting which models are enabled for a CLI provider. */
export function ModelCheckboxList({
  modelOptions,
  enabledModels,
  disabled,
  thinkingOptions,
  thinkingLevels,
  thinkingTriggerLabels,
  onToggleModel,
  onToggleAll,
  onThinkingChange,
}: ModelCheckboxListProps): ReactNode {
  const allSelected = modelOptions.every((opt) => enabledModels.has(opt.value));
  const hasAnyThinking = thinkingOptions !== undefined
    && Object.values(thinkingOptions).some((opts) => opts.length > 0);

  return (
    <div className="mt-4">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-[10px] font-medium uppercase tracking-wider text-fg-faint">
          Models
        </p>
        <div className="flex items-center gap-3">
          {hasAnyThinking && (
            <p className="text-[10px] font-medium uppercase tracking-wider text-fg-faint">
              Thinking
            </p>
          )}
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
      </div>
      <div className="flex flex-col gap-1.5">
        {modelOptions.map((opt) => {
          const isChecked = enabledModels.has(opt.value);
          return (
            <div key={opt.value} className="flex items-center gap-2">
              <button
                type="button"
                onClick={() => onToggleModel(opt.value)}
                disabled={disabled}
                className={`flex min-w-0 flex-1 items-center gap-2.5 rounded-md px-3 py-2 text-left text-sm transition-colors ${
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
                <TruncatedLabel text={opt.label} />
              </button>
              {(() => {
                const modelOpts = thinkingOptions?.[opt.value];
                if (!modelOpts || modelOpts.length === 0) return null;
                return (
                  <div className="shrink-0">
                    <PopoverSelect
                      value={thinkingLevels[opt.value] ?? ""}
                      options={modelOpts}
                      onChange={(v) => onThinkingChange(opt.value, v)}
                      disabled={disabled}
                      direction="down"
                      placeholder="Default"
                      triggerClassName={THINKING_TRIGGER_CLASS}
                      menuClassName={THINKING_MENU_CLASS}
                      triggerLabels={thinkingTriggerLabels}
                      menuTitle="Thinking Level"
                    />
                  </div>
                );
              })()}
            </div>
          );
        })}
      </div>
    </div>
  );
}
