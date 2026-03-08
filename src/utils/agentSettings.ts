import type { AppSettings } from "../types";

export const MINIMUM_AGENT_FIELDS = [
  "promptEnhancerAgent",
  "generatorAgent",
  "reviewerAgent",
  "fixerAgent",
  "finalJudgeAgent",
  "executiveSummaryAgent",
] as const;

const MINIMUM_AGENT_LABELS: Record<(typeof MINIMUM_AGENT_FIELDS)[number], string> = {
  promptEnhancerAgent: "Prompt Enhancer",
  generatorAgent: "Coder",
  reviewerAgent: "Code Reviewer",
  fixerAgent: "Code Fixer",
  finalJudgeAgent: "Judge",
  executiveSummaryAgent: "Executive Summary",
};

export function missingMinimumAgentLabels(settings: AppSettings): string[] {
  return MINIMUM_AGENT_FIELDS
    .filter((field) => !settings[field])
    .map((field) => MINIMUM_AGENT_LABELS[field]);
}

export function hasMinimumAgentsConfigured(settings: AppSettings): boolean {
  return missingMinimumAgentLabels(settings).length === 0;
}
