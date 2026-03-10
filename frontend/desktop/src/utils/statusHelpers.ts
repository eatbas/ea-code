import type { PipelineStatus } from "../types";

/** Whether the pipeline is actively executing. */
export function isActive(status: PipelineStatus): boolean {
  return status === "running" || status === "waiting_for_input";
}

/** Whether the pipeline is in a terminal state. */
export function isTerminal(status: PipelineStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

/** Status label and colour for the current pipeline state. */
export function statusInfo(status: PipelineStatus): { label: string; colour: string } {
  switch (status) {
    case "running":
      return { label: "Running", colour: "#22c55e" };
    case "paused":
      return { label: "Paused", colour: "#60a5fa" };
    case "waiting_for_input":
      return { label: "Awaiting input", colour: "#f59e0b" };
    case "completed":
      return { label: "Completed", colour: "#22c55e" };
    case "failed":
      return { label: "Failed", colour: "#ef4444" };
    case "cancelled":
      return { label: "Cancelled", colour: "#f59e0b" };
    default:
      return { label: "Idle", colour: "#9898b0" };
  }
}
