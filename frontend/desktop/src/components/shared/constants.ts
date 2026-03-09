import type { AgentBackend, PipelineStage } from "../../types";

/** Agent backend options for dropdown selects. */
export const BACKEND_OPTIONS: { value: AgentBackend; label: string }[] = [
  { value: "claude", label: "Claude" },
  { value: "codex", label: "Codex" },
  { value: "gemini", label: "Gemini" },
  { value: "kimi", label: "Kimi" },
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
  direct_task: "Direct Task",
};

/** Stage badge background colours (rgba strings). */
export const STAGE_COLOURS: Record<PipelineStage, string> = {
  prompt_enhance: "rgba(100, 200, 255, 0.22)",
  skill_select: "rgba(70, 205, 125, 0.24)",
  plan: "rgba(64, 196, 255, 0.24)",
  plan_audit: "rgba(255, 196, 64, 0.24)",
  generate: "rgba(90, 140, 255, 0.25)",
  diff_after_generate: "rgba(150, 150, 150, 0.22)",
  review: "rgba(255, 180, 50, 0.22)",
  fix: "rgba(180, 100, 255, 0.22)",
  diff_after_fix: "rgba(150, 150, 150, 0.22)",
  judge: "rgba(255, 100, 100, 0.22)",
  executive_summary: "rgba(0, 200, 80, 0.3)",
  direct_task: "rgba(90, 140, 255, 0.25)",
};

/** Display labels for artefact kinds. */
export const ARTIFACT_LABELS: Record<string, string> = {
  diff: "Diff",
  plan: "Plan",
  plan_audit: "Plan Audit",
  plan_final: "Final Plan",
  plan_revised: "Revised Plan",
  review: "Review",
  judge: "Judge Verdict",
  executive_summary: "Summary",
  selected_skills: "Selected Skills",
  workspace_context: "Workspace Context",
  result: "Result",
};
