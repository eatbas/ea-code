import type { ReactNode } from "react";
import type { AppSettings, ProviderInfo } from "../../types";
import { CascadingSelect } from "./CascadingSelect";

/** Extra settings keys that should be written in lockstep with the primary pair. */
export interface GroupedKeys {
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
}

/** Props for a single stage card. */
export interface StageCardProps {
  label: string;
  tag: string;
  /** Optional subtitle shown below the label. */
  subtitle?: string;
  backendKey: keyof AppSettings;
  modelKey: keyof AppSettings;
  /** Additional settings keys written in lockstep (for grouped cards). */
  groupedKeys?: GroupedKeys[];
  children?: ReactNode;
  optional: boolean;
  draft: AppSettings;
  providers: ProviderInfo[];
  providersLoading: boolean;
  onUpdate: (patch: Partial<AppSettings>) => void;
  onRemove?: () => void;
}

/** A single pipeline stage agent card with optional remove button. */
export function StageCard({
  label, tag, subtitle, backendKey, modelKey, groupedKeys, optional,
  children, draft, providers, providersLoading, onUpdate, onRemove,
}: StageCardProps): ReactNode {
  const currentBackend = draft[backendKey] as string | null;
  const currentModel = draft[modelKey] as string | null;
  const isMandatoryUnconfigured = !optional && (!currentBackend || !currentModel);

  return (
    <div className="relative rounded-lg border border-[#2e2e48] bg-[#1a1a24] p-4 flex flex-col gap-2">
      {isMandatoryUnconfigured && (
        <span aria-hidden="true" className="absolute inset-y-0 left-0 w-1.5 rounded-l-lg bg-[#dc2626]" />
      )}
      <div className="flex items-center justify-between">
        <div>
          <span className="text-xs font-medium text-[#9898b0]">
            {label}
            <span className="ml-1 text-[#6b6b80]">{tag}</span>
          </span>
          {subtitle && (
            <p className="mt-0.5 text-[10px] text-[#6b6b80]">{subtitle}</p>
          )}
        </div>
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
        providers={providers}
        providersLoading={providersLoading}
        onChange={(newBackend, newModel) => {
          const patch: Record<string, unknown> = {
            [backendKey]: newBackend,
            [modelKey]: newModel ?? "",
          };
          // Write grouped keys in lockstep.
          if (groupedKeys) {
            for (const gk of groupedKeys) {
              patch[gk.backendKey as string] = newBackend;
              patch[gk.modelKey as string] = newModel ?? "";
            }
          }
          onUpdate(patch as Partial<AppSettings>);
        }}
      />
      {children}
    </div>
  );
}
