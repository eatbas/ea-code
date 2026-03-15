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
  prompt_enhance: "Enhancing Prompt",
  skill_select: "Skills",
  plan: "Planning",
  plan_audit: "Auditing Plan",
  coder: "Coding",
  code_reviewer: "Code Review",
  code_fixer: "Code Fix",
  judge: "Judge",
  executive_summary: "Summary",
  direct_task: "Direct Task",
};

/** Stage badge background classes. */
export const STAGE_BADGE_CLASSES: Record<PipelineStage, string> = {
  prompt_enhance: "bg-[#64c8ff]/20",
  skill_select: "bg-[#46cd7d]/25",
  plan: "bg-[#40c4ff]/25",
  plan_audit: "bg-[#ffc440]/25",
  coder: "bg-[#5a8cff]/25",
  code_reviewer: "bg-[#ffb432]/20",
  code_fixer: "bg-[#b464ff]/20",
  judge: "bg-[#ff6464]/20",
  executive_summary: "bg-[#00c850]/30",
  direct_task: "bg-[#5a8cff]/25",
};

/** Display labels for artifact kinds (kept for runtime artifact display during live runs). */
export const ARTIFACT_LABELS: Record<string, string> = {
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
