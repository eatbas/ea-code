import type { ReactNode } from "react";
import type { AppSettings, StageResult } from "../../types";
import { stageModelLabel } from "../../utils/stageModelLabels";
import { StageInputOutputCard } from "./StageInputOutputCard";
import { StageCard } from "./StageCard";

interface RichStageCardProps {
  stage: StageResult;
  runPrompt: string;
  enhancedPromptInput: string;
  promptEnhanceOutput: string;
  planOutput: string;
  planInputForAudit: string;
  auditedPlanOutput: string;
  settings: AppSettings | null;
  startedAt?: number;
  showPlanCard?: boolean;
  showPlanAuditCard?: boolean;
}

/** Renders the rich stage cards shared by live and historical run views. */
export function RichStageCard({
  stage,
  runPrompt,
  enhancedPromptInput,
  promptEnhanceOutput,
  planOutput,
  planInputForAudit,
  auditedPlanOutput,
  settings,
  startedAt,
  showPlanCard = true,
  showPlanAuditCard = true,
}: RichStageCardProps): ReactNode {
  if (stage.stage === "prompt_enhance" && stage.status === "completed") {
    return (
      <StageInputOutputCard
        title="Enhancing Prompt"
        inputSections={[{ label: "Original Prompt", content: runPrompt }]}
        outputLabel="Result"
        outputContent={promptEnhanceOutput || "No valid enhanced prompt output generated."}
        modelLabel={stageModelLabel("prompt_enhance", settings)}
        durationMs={stage.durationMs}
        badgeClassName="bg-emerald-400/25"
        outputClassName="border border-emerald-400/20 bg-emerald-400/5 text-[#e4e4ed]"
      />
    );
  }

  if (stage.stage === "plan" && stage.status === "completed" && showPlanCard) {
    return (
      <StageInputOutputCard
        title="Planning"
        inputSections={[{ label: "Original Prompt", content: runPrompt }, { label: "Enhanced Prompt", content: enhancedPromptInput }]}
        outputLabel="Plan"
        outputContent={planOutput || "No valid plan output generated."}
        modelLabel={stageModelLabel("plan", settings)}
        durationMs={stage.durationMs}
        badgeClassName="bg-sky-400/25"
      />
    );
  }

  if (stage.stage === "plan_audit" && stage.status === "completed" && showPlanAuditCard) {
    return (
      <StageInputOutputCard
        title="Auditing Plan"
        inputSections={[{ label: "Original Prompt", content: runPrompt }, { label: "Enhanced Prompt", content: enhancedPromptInput }, { label: "Plan", content: planInputForAudit }]}
        outputLabel="Audited Plan"
        outputContent={auditedPlanOutput || "No valid audited plan output generated."}
        modelLabel={stageModelLabel("plan_audit", settings)}
        durationMs={stage.durationMs}
        badgeClassName="bg-amber-400/25"
        outputClassName="border border-amber-400/20 bg-amber-400/5 text-[#e4e4ed]"
      />
    );
  }

  return (
    <StageCard
      stage={stage}
      modelLabel={stageModelLabel(stage.stage, settings)}
      startedAt={startedAt}
    />
  );
}
