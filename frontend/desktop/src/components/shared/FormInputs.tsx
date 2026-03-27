import type { ReactNode } from "react";
import type { ProviderInfo } from "../../types";
import { backendOptionsFromProviders } from "./constants";

/** Props for the TextInput component. */
export interface TextInputProps {
  label: string;
  value: string;
  onChange: (v: string) => void;
}

/** Reusable text input row for settings forms. */
export function TextInput({ label, value, onChange }: TextInputProps): ReactNode {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#8b8b93]">{label}</span>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="rounded border border-[#313134] bg-[#0b0b0c] px-3 py-1.5 text-sm text-[#f5f5f5] focus:border-[#68686f] focus:outline-none"
      />
    </label>
  );
}

/** Props for the AgentSelect component. */
export interface AgentSelectProps {
  label: string;
  value: string | null;
  providers: ProviderInfo[];
  onChange: (v: string | null) => void;
}

/** Reusable select dropdown row for agent role mapping. */
export function AgentSelect({ label, value, providers, onChange }: AgentSelectProps): ReactNode {
  const options = backendOptionsFromProviders(providers.filter((p) => p.available));
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#8b8b93]">{label}</span>
      <select
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
        className="rounded border border-[#313134] bg-[#0b0b0c] px-3 py-1.5 text-sm text-[#f5f5f5] focus:border-[#68686f] focus:outline-none"
      >
        <option value="">Not selected</option>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Props for the OptionalAgentSelect component. */
export interface OptionalAgentSelectProps {
  label: string;
  value: string | null;
  providers: ProviderInfo[];
  onChange: (v: string | null) => void;
}

/** Reusable optional select row with an explicit skip option. */
export function OptionalAgentSelect({ label, value, providers, onChange }: OptionalAgentSelectProps): ReactNode {
  const options = backendOptionsFromProviders(providers.filter((p) => p.available));
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs font-medium text-[#8b8b93]">{label}</span>
      <select
        value={value ?? ""}
        onChange={(e) => onChange(e.target.value === "" ? null : e.target.value)}
        className="rounded border border-[#313134] bg-[#0b0b0c] px-3 py-1.5 text-sm text-[#f5f5f5] focus:border-[#68686f] focus:outline-none"
      >
        <option value="">Not selected (Skip)</option>
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/** Props for the HealthDot component. */
export interface HealthDotProps {
  available: boolean;
  error?: string;
}

/** Health status indicator dot. */
export function HealthDot({ available, error }: HealthDotProps): ReactNode {
  return (
    <span
      title={error ?? (available ? "Available" : "Not found")}
      className={`inline-block h-2.5 w-2.5 rounded-full ${
        available ? "bg-[#22c55e]" : "bg-[#ef4444]"
      }`}
    />
  );
}
