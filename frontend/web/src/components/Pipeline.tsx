import {
  Sparkles,
  ClipboardList,
  GitMerge,
  Code2,
  MessageSquare,
  Wrench,
  RotateCcw,
} from "lucide-react";
import type { ReactNode } from "react";

interface StageProps {
  icon: ReactNode;
  label: string;
  description: string;
  index: number;
  highlight?: boolean;
}

const STAGES = [
  { icon: <Sparkles className="h-4 w-4" />, label: "Orchestrator", description: "Analyses and enhances your prompt for clarity, then generates a short summary title for the conversation. Optional — can be disabled in settings." },
  { icon: <ClipboardList className="h-4 w-4" />, label: "Planner x3", description: "Up to three agents draft implementation plans in parallel — multiple perspectives, not one guess.", highlight: true },
  { icon: <GitMerge className="h-4 w-4" />, label: "Plan Merge", description: "Consolidates all planner outputs into a single coherent plan. You review and approve it before any code is written." },
  { icon: <Code2 className="h-4 w-4" />, label: "Coder", description: "Implements the approved plan, modifying your codebase and signalling completion when done." },
  { icon: <MessageSquare className="h-4 w-4" />, label: "Reviewer x3", description: "Up to three agents review the code in parallel using git diff — no model marks its own homework.", highlight: true },
  { icon: <GitMerge className="h-4 w-4" />, label: "Review Merge", description: "Deduplicates and prioritises all reviewer findings into a single consolidated report by severity." },
  { icon: <Wrench className="h-4 w-4" />, label: "Code Fixer", description: "Applies all critical and major fixes from the merged review, resuming the coder's session for full context." },
];

function Stage({ icon, label, description, index, highlight }: StageProps) {
  return (
    <div className="group relative flex items-start gap-4">
      {/* Connector line */}
      {index < STAGES.length - 1 && (
        <span className="absolute left-5 top-12 h-full w-px bg-gradient-to-b from-accent/30 to-border" />
      )}

      {/* Icon circle */}
      <span className={`relative z-10 flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border transition-colors duration-200 group-hover:border-accent group-hover:bg-accent-soft ${highlight ? "border-accent bg-accent-soft text-accent" : "border-border bg-white text-accent"}`}>
        {icon}
      </span>

      {/* Text */}
      <div className="pb-8">
        <div className="flex items-center gap-2">
          <h4 className="text-sm font-semibold text-heading">{label}</h4>
          {highlight && (
            <span className="rounded-full bg-accent-soft px-2 py-0.5 font-mono text-[10px] font-bold uppercase tracking-wider text-accent">
              Parallel
            </span>
          )}
        </div>
        <p className="mt-1 text-sm leading-relaxed text-muted">{description}</p>
      </div>
    </div>
  );
}

export function Pipeline() {
  return (
    <section id="pipeline" className="relative bg-white py-24 md:py-32">
      <div className="mx-auto max-w-6xl px-6">
        <div className="grid gap-16 lg:grid-cols-2 lg:gap-20">
          {/* Left — headline */}
          <div className="fade-in-up lg:sticky lg:top-32 lg:self-start">
            <p className="mb-3 font-mono text-xs font-medium uppercase tracking-widest text-accent">
              The Pipeline
            </p>
            <h2 className="text-3xl font-bold tracking-tight text-heading md:text-4xl">
              7 Stages.
              <br />
              Multiple Agents.
              <br />
              Human in the Loop.
            </h2>
            <p className="mt-4 max-w-md text-muted leading-relaxed">
              Each run flows through up to seven stages. Planning and review
              happen in parallel with multiple agents. You approve the plan
              before any code is written, and can trigger as many review
              cycles as you need.
            </p>

            {/* Redo review badge */}
            <div className="mt-8 inline-flex items-center gap-3 rounded-xl border border-border bg-surface-elevated px-5 py-3">
              <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent-soft text-accent">
                <RotateCcw className="h-4 w-4" />
              </span>
              <span className="text-sm text-muted">
                Redo Review — re-review and fix as many times as needed
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
