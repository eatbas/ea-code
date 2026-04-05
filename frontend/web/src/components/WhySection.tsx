import { X, Check } from "lucide-react";

interface ComparisonRowProps {
  single: string;
  ea: string;
}

function ComparisonRow({ single, ea }: ComparisonRowProps) {
  return (
    <div className="grid grid-cols-2 gap-4 border-b border-border-dark py-4 last:border-b-0">
      <div className="flex items-start gap-3">
        <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-red-500/15 text-red-400">
          <X className="h-3 w-3" />
        </span>
        <span className="text-sm leading-relaxed text-faint">{single}</span>
      </div>
      <div className="flex items-start gap-3">
        <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-accent/15 text-accent">
          <Check className="h-3 w-3" />
        </span>
        <span className="text-sm leading-relaxed text-white">{ea}</span>
      </div>
    </div>
  );
}

const COMPARISONS: ComparisonRowProps[] = [
  {
    single: "One model plans, codes, reviews, and judges its own work",
    ea: "Different agents specialise in each role",
  },
  {
    single: "Blind spots go unnoticed",
    ea: "Parallel reviewers catch what one misses",
  },
  {
    single: "You manually re-prompt when output is wrong",
    ea: "Redo Review cycles re-review and fix with full context",
  },
  {
    single: "No review before implementation begins",
    ea: "Plan approval gates let you review before any code is written",
  },
  {
    single: "You pay for 5 subscriptions and use 1 at a time",
    ea: "Every subscription earns its keep",
  },
];

export function WhySection() {
  return (
    <section id="why" className="relative bg-surface-dark py-24 md:py-32">
      <div className="mx-auto max-w-4xl px-6">
        <div className="fade-in-up mb-14 text-center">
          <p className="mb-3 font-mono text-xs font-medium uppercase tracking-widest text-accent">
            The Difference
          </p>
          <h2 className="text-3xl font-bold tracking-tight text-white md:text-4xl">
            One Agent vs. <span className="text-accent">Many</span>
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-faint leading-relaxed">
            When a single model plans, codes, reviews, and judges its own
            work, mistakes compound. Maestro splits the job across
            multiple agents so each one keeps the others honest.
          </p>
        </div>

        <div className="fade-in-up rounded-2xl border border-border-dark bg-white/5 p-6 md:p-8">
          {/* Header */}
          <div className="mb-2 grid grid-cols-2 gap-4 border-b border-border-dark pb-4">
            <span className="font-mono text-xs font-bold uppercase tracking-widest text-red-400">
              Single Agent
            </span>
            <span className="font-mono text-xs font-bold uppercase tracking-widest text-accent">
              Maestro
            </span>
          </div>

          {/* Rows */}
          {COMPARISONS.map((row) => (
            <ComparisonRow key={row.single} {...row} />
          ))}
        </div>
      </div>
    </section>
  );
}
