import type { McpRuntimeStatus } from "../../types";

export const AI_CLI_ORDER = ["claude", "codex", "gemini", "kimi", "opencode"] as const;

export function dotColourClass(status: McpRuntimeStatus): string {
  switch (status) {
    case "enabled":
      return "bg-[#22c55e]";
    case "disabled":
    case "unknown":
      return "bg-[#f59e0b]";
    case "error":
      return "bg-[#ef4444]";
    case "notInstalled":
    default:
      return "bg-[#64748b]";
  }
}

export function dotTooltip(status: McpRuntimeStatus, message?: string): string {
  switch (status) {
    case "enabled":
      return "Enabled";
    case "disabled":
      return message ?? "Disabled";
    case "unknown":
      return message ?? "Unknown status";
    case "error":
      return message ?? "Error";
    case "notInstalled":
    default:
      return "Not installed";
  }
}

/** Returns true when the CLI is installed but the MCP server status is problematic. */
export function isFixable(status: McpRuntimeStatus): boolean {
  return status === "disabled" || status === "unknown" || status === "error";
}

export function parseEnv(raw: string): Record<string, string> {
  try {
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return {};
  }
}
