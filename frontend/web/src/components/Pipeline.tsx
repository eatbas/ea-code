import {
  Sparkles,
  BookOpen,
  ClipboardList,
  ShieldCheck,
  Code2,
  MessageSquare,
  GitMerge,
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
  highlight?: boolean;
}

function Stage({ icon, label, description, index, highlight }: StageProps) {
  return (
    <div className="group relative flex items-start gap-4">
      {/* Connector line */}
      {index < STAGES.length - 1 && (
        <span className="absolute left-5 top-12 h-full w-px bg-gradient-to-b from-accent/40 to-transparent" />
      )}

      {/* Icon circle */}
      <span className={`relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border transition-colors duration-200 group-hover:border-accent/50 group-hover:bg-accent/10 ${highlight ? "border-accent/40 bg-accent/10 text-accent" : "border-border bg-surface-elevated text-accent"}`}>
        {icon}
      </span>

      {/* Text */}
      <div className="pb-8">
        <div className="flex items-center gap-2">
          <h4 className="font-mono text-sm font-semibold">{label}</h4>
          {highlight && (
            <span className="rounded-full bg-accent/15 px-2 py-0.5 font-mono text-[10px] font-bold uppercase tracking-wider text-accent">
              Parallel
            </span>
          )}
        </div>
        <p className="mt-1 text-sm leading-relaxed text-muted">{description}</p>
      </div>
    </div>
  );
}

const STAGES = [
  { icon: <Sparkles className="h-4 w-4" />, label: "Prompt Enhance", description: "Sharpens your natural-language prompt into a precise, context-aware instruction." },
  { icon: <BookOpen className="h-4 w-4" />, label: "Skill Select", description: "Pulls relevant guidance from your skills catalogue so agents have domain context." },
  { icon: <ClipboardList className="h-4 w-4" />, label: "Plan x3", description: "Up to 3 agents draft implementation plans in parallel — multiple perspectives, not one guess.", highlight: true },
  { icon: <ShieldCheck className="h-4 w-4" />, label: "Plan Audit", description: "A different agent pressure-tests the plan for gaps, risks, and edge cases before any code is written." },
  { icon: <Code2 className="h-4 w-4" />, label: "Code", description: "Writes or modifies code according to the approved plan." },
  { icon: <MessageSquare className="h-4 w-4" />, label: "Review x3", description: "Up to 3 agents review the code in parallel — no model marks its own homework.", highlight: true },
  { icon: <GitMerge className="h-4 w-4" />, label: "Review Merge", description: "Combines independent reviewer findings into a single, unified set of feedback." },
  { icon: <Wrench className="h-4 w-4" />, label: "Fix", description: "Applies all review feedback and resolves every flagged issue." },
  { icon: <Gavel className="h-4 w-4" />, label: "Judge", description: "Final verdict — COMPLETE and ship, or loop back with full context for another iteration." },
  { icon: <FileText className="h-4 w-4" />, label: "Executive Summary", description: "Records exactly what happened, what changed, and why." },
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
              10 Stages.
              <br />
              Multiple Agents.
              <br />
              Self-Improving.
            </h2>
            <p className="mt-4 max-w-md text-muted leading-relaxed">
              Each run flows through up to 10 stages. Planning and review
              happen in parallel with multiple agents. If the Judge says
              &ldquo;not done,&rdquo; Maestro loops back automatically —
              refining, regenerating, and reviewing until the job is truly
              complete.
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
