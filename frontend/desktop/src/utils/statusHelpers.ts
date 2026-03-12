import type { PipelineRun, PipelineStatus } from "../types";

type StatusTone = "success" | "info" | "warning" | "danger" | "neutral";

interface StatusToneClasses {
  text: string;
  badge: string;
  dot: string;
  cardBg: string;
  cardBorder: string;
}

/** Whether the pipeline is actively executing. */
export function isActive(status: PipelineStatus): boolean {
  return status === "running" || status === "waiting_for_input";
}

/** Whether the pipeline is in a terminal state. */
export function isTerminal(status: PipelineStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

/** Whether a run exists and is in a non-terminal, user-visible execution state. */
export function isRunInProgress(run: PipelineRun | null): boolean {
  return !!run && (run.status === "running" || run.status === "waiting_for_input" || run.status === "paused");
}

/** Whether a run exists and has reached a terminal state. */
export function isRunTerminalState(run: PipelineRun | null): boolean {
  return !!run && isTerminal(run.status);
}

/** Whether a persisted session status indicates active/live execution. */
export function isLiveSessionStatus(status: string | undefined): boolean {
  return status === "running" || status === "waiting_for_input" || status === "paused";
}

/** Whether an untyped status string is actively running work (excludes paused). */
export function isActiveStatusValue(status: string | undefined): boolean {
  return status === "running" || status === "waiting_for_input";
}

/** Whether an untyped status string is terminal. */
export function isTerminalStatusValue(status: string | undefined): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

/** Maps pipeline statuses to a semantic tone used by the UI. */
export function statusTone(status: PipelineStatus | string | undefined): StatusTone {
  if (status === "running" || status === "completed") return "success";
  if (status === "paused") return "info";
  if (status === "waiting_for_input" || status === "cancelled") return "warning";
  if (status === "failed") return "danger";
  return "neutral";
}

/** Returns reusable Tailwind class sets for status-based UI styling. */
export function statusToneClasses(status: PipelineStatus | string | undefined): StatusToneClasses {
  switch (statusTone(status)) {
    case "success":
      return {
        text: "text-[#22c55e]",
        badge: "text-[#22c55e] bg-[#22c55e]/10",
        dot: "bg-[#22c55e]",
        cardBg: "bg-[#22c55e]/10",
        cardBorder: "border-[#22c55e]/30",
      };
    case "info":
      return {
        text: "text-[#60a5fa]",
        badge: "text-[#60a5fa] bg-[#60a5fa]/10",
        dot: "bg-[#60a5fa]",
        cardBg: "bg-[#60a5fa]/10",
        cardBorder: "border-[#60a5fa]/30",
      };
    case "warning":
      return {
        text: "text-[#f59e0b]",
        badge: "text-[#f59e0b] bg-[#f59e0b]/10",
        dot: "bg-[#f59e0b]",
        cardBg: "bg-[#f59e0b]/10",
        cardBorder: "border-[#f59e0b]/30",
      };
    case "danger":
      return {
        text: "text-[#ef4444]",
        badge: "text-[#ef4444] bg-[#ef4444]/10",
        dot: "bg-[#ef4444]",
        cardBg: "bg-[#ef4444]/10",
        cardBorder: "border-[#ef4444]/30",
      };
    default:
      return {
        text: "text-[#9898b0]",
        badge: "text-[#9898b0] bg-[#9898b0]/10",
        dot: "bg-[#9898b0]",
        cardBg: "bg-[#1a1a24]",
        cardBorder: "border-[#2e2e48]",
      };
  }
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
