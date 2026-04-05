import {
  Layers,
  RotateCcw,
  Activity,
  ShieldCheck,
  History,
  Bug,
} from "lucide-react";
import type { ReactNode } from "react";

interface FeatureCardProps {
  icon: ReactNode;
  title: string;
  description: string;
  wide?: boolean;
}

function FeatureCard({ icon, title, description, wide }: FeatureCardProps) {
  return (
    <div
      className={`accent-line group rounded-2xl border border-border-dark bg-white/5 p-6 transition-all duration-200 hover:border-accent/40 hover:bg-white/10 cursor-pointer ${
        wide ? "md:col-span-2" : ""
      }`}
    >
      <span className="mb-4 inline-flex h-10 w-10 items-center justify-center rounded-xl bg-accent/10 text-accent transition-colors duration-200 group-hover:bg-accent group-hover:text-white">
        {icon}
      </span>
      <h3 className="text-base font-semibold text-white">{title}</h3>
      <p className="mt-2 text-sm leading-relaxed text-faint">{description}</p>
    </div>
  );
}

const FEATURES: FeatureCardProps[] = [
  {
    icon: <Layers className="h-5 w-5" />,
    title: "Mix & Match Agents",
    description:
      "Assign Claude for planning, Codex for code generation, Gemini for review — every subscription earns its keep.",
    wide: true,
  },
  {
    icon: <RotateCcw className="h-5 w-5" />,
    title: "Redo Review Cycles",
    description:
      "Not satisfied? Trigger another review cycle. Reviewers re-inspect, merge feedback, and the Code Fixer applies fixes — repeat as many times as needed.",
  },
  {
    icon: <Activity className="h-5 w-5" />,
    title: "Real-Time Tracking",
    description:
      "Watch every stage execute live — plans, reviews, and diffs stream in as they happen with a full debug log viewer.",
  },
  {
    icon: <ShieldCheck className="h-5 w-5" />,
    title: "Plan Approval Gates",
    description:
      "Pause after Plan Merge to review, edit, or provide feedback on the plan — no code is written until you approve.",
  },
  {
    icon: <History className="h-5 w-5" />,
    title: "Session History",
    description:
      "Every run, iteration, and artefact is persisted locally under ~/.maestro/ for full traceability. No cloud required.",
  },
  {
    icon: <Bug className="h-5 w-5" />,
    title: "Debug Log Viewer",
    description:
      "Collapsible debug panel shows the full pipeline execution trace in real time. Copy the entire log with one click.",
  },
];

export function Features() {
  return (
    <section id="features" className="relative bg-surface-dark py-24 dot-grid-dark md:py-32">
      <div className="mx-auto max-w-6xl px-6">
        <div className="fade-in-up mb-14 text-center">
          <p className="mb-3 font-mono text-xs font-medium uppercase tracking-widest text-accent">
            Features
          </p>
          <h2 className="text-3xl font-bold tracking-tight text-white md:text-4xl">
            One Control Room for Every Agent
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-faint leading-relaxed">
            Stop switching between terminals and re-explaining context.
            Maestro gives you a single interface to configure, run, and
            monitor multi-agent development workflows.
          </p>
        </div>

        <div className="stagger grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map((f) => (
            <FeatureCard key={f.title} {...f} />
          ))}
        </div>
      </div>
    </section>
  );
}
