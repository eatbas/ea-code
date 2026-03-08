import type { AgentBackend, PipelineStage } from "../../types";

/** Agent backend options for dropdown selects. */
export const BACKEND_OPTIONS: { value: AgentBackend; label: string }[] = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini" },
  { value: "kimi", label: "Kimi" },
  { value: "copilot", label: "Copilot" },
  { value: "opencode", label: "OpenCode" },
];

/** Display labels for each pipeline stage. */
export const STAGE_LABELS: Record<PipelineStage, string> = {
  prompt_enhance: "Prompt",
  skill_select: "Skills",
  plan: "Plan",
  plan_audit: "Plan Audit",
  generate: "Generate",
  diff_after_generate: "Diff",
  review: "Review",
  fix: "Fix",
  diff_after_fix: "Diff",
  judge: "Judge",
  executive_summary: "Summary",
};
