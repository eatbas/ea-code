import type { AgentSelection } from "../types";

/** Serialise an AgentSelection to the `"provider:model"` settings format. */
export function serialiseAgentSelection(agent: AgentSelection): string {
  return `${agent.provider}:${agent.model}`;
}

/** Parse a `"provider:model"` settings string back to an AgentSelection, or null if invalid. */
export function parseAgentSelection(value: string | null): AgentSelection | null {
  if (!value) return null;
  const separatorIndex = value.indexOf(":");
  if (separatorIndex < 1) return null;
  return {
    provider: value.slice(0, separatorIndex),
    model: value.slice(separatorIndex + 1),
  };
}
