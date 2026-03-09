import {
  Sparkles,
  BookOpen,
  ClipboardList,
  ShieldCheck,
  Code2,
  MessageSquare,
  Wrench,
  Gavel,
  FileText,
} from "lucide-react";
import type { ReactNode } from "react";

interface StageProps {
  icon: ReactNode;
  label: string;
  description: string;
  index: number;
}

function Stage({ icon, label, description, index }: StageProps) {
  return (
    <div className="group relative flex items-start gap-4">
      {/* Connector line */}
      {index < STAGES.length - 1 && (
        <span className="absolute left-5 top-12 h-full w-px bg-gradient-to-b from-accent/40 to-transparent" />
      )}

      {/* Icon circle */}
      <span className="relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-border bg-surface-elevated text-accent transition-colors duration-200 group-hover:border-accent/50 group-hover:bg-accent/10">
        {icon}
      </span>

      {/* Text */}
      <div className="pb-8">
        <h4 className="font-mono text-sm font-semibold">{label}</h4>
        <p className="mt-1 text-sm leading-relaxed text-muted">{description}</p>
      </div>
    </div>
  );
}

const STAGES = [
  { icon: <Sparkles className="h-4 w-4" />, label: "Prompt Enhance", description: "Refines your natural-language prompt into a precise, context-aware instruction." },
  { icon: <BookOpen className="h-4 w-4" />, label: "Skill Select", description: "Matches your request against a curated skills catalogue for domain-specific guidance." },
  { icon: <ClipboardList className="h-4 w-4" />, label: "Plan", description: "Generates a step-by-step execution plan with file targets and acceptance criteria." },
  { icon: <ShieldCheck className="h-4 w-4" />, label: "Plan Audit", description: "A second agent reviews the plan for gaps, risks, and edge cases." },
  { icon: <Code2 className="h-4 w-4" />, label: "Generate", description: "Writes or modifies code according to the approved plan." },
  { icon: <MessageSquare className="h-4 w-4" />, label: "Review", description: "Automated code review catches bugs, style issues, and missed requirements." },
  { icon: <Wrench className="h-4 w-4" />, label: "Fix", description: "Applies review suggestions and resolves any flagged issues." },
  { icon: <Gavel className="h-4 w-4" />, label: "Judge", description: "Final verdict — COMPLETE or loop back for another iteration." },
  { icon: <FileText className="h-4 w-4" />, label: "Executive Summary", description: "Generates a concise report of everything that was done and why." },
];

export function Pipeline() {
  return (
    <section id="pipeline" className="relative py-24 md:py-32">
      <div className="mx-auto max-w-6xl px-6">
        <div className="grid gap-16 lg:grid-cols-2 lg:gap-20">
          {/* Left — headline */}
          <div className="fade-in-up lg:sticky lg:top-32 lg:self-start">
            <p className="mb-3 font-mono text-xs font-medium uppercase tracking-widest text-accent">
              The Pipeline
            </p>
            <h2 className="font-mono text-3xl font-bold tracking-tight md:text-4xl">
              A Self-Improving
              <br />
              Dev Loop
            </h2>
            <p className="mt-4 max-w-md text-muted leading-relaxed">
              Each run flows through up to 9 stages. If the Judge says
              &ldquo;not done,&rdquo; EA Code loops back automatically —
              refining the prompt, regenerating code, and reviewing again until
              your task is truly complete.
            </p>

            {/* Loop badge */}
            <div className="mt-8 inline-flex items-center gap-3 rounded-xl border border-border bg-surface-elevated px-5 py-3">
              <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent/15 font-mono text-xs font-bold text-accent">
                3x
              </span>
              <span className="text-sm text-muted">
                Default max iterations — fully configurable
              </span>
            </div>
          </div>

          {/* Right — stages timeline */}
          <div className="stagger">
            {STAGES.map((stage, i) => (
              <Stage key={stage.label} index={i} {...stage} />
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
