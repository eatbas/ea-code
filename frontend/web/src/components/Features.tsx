import {
  Layers,
  RefreshCcw,
  Activity,
  Puzzle,
  History,
  ShieldCheck,
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
      className={`glow-border group rounded-2xl border border-border bg-surface-elevated p-6 transition-colors duration-200 hover:border-accent/30 cursor-pointer ${
        wide ? "md:col-span-2" : ""
      }`}
    >
      <span className="mb-4 inline-flex h-10 w-10 items-center justify-center rounded-xl bg-accent/10 text-accent transition-colors duration-200 group-hover:bg-accent/20">
        {icon}
      </span>
      <h3 className="font-mono text-base font-semibold">{title}</h3>
      <p className="mt-2 text-sm leading-relaxed text-muted">{description}</p>
    </div>
  );
}

const FEATURES: FeatureCardProps[] = [
  {
    icon: <Layers className="h-5 w-5" />,
    title: "Mix & Match Agents",
    description:
      "Assign Claude for planning, Codex for code generation, Gemini for review — use each model where it shines.",
    wide: true,
  },
  {
    icon: <RefreshCcw className="h-5 w-5" />,
    title: "Self-Improving Loop",
    description:
      "The Judge stage decides if the task is done. If not, the pipeline loops back and iterates automatically.",
  },
  {
    icon: <Activity className="h-5 w-5" />,
    title: "Real-Time Tracking",
    description:
      "Watch every stage execute live — diffs, plans, reviews, and verdicts appear as they happen.",
  },
  {
    icon: <Puzzle className="h-5 w-5" />,
    title: "Skills & MCP Servers",
    description:
      "Create custom skills and connect MCP servers for domain-specific context, external tools, and knowledge bases.",
  },
  {
    icon: <History className="h-5 w-5" />,
    title: "Session History",
    description:
      "Every run, iteration, artefact, and question is persisted to a local SQLite database for full traceability.",
  },
  {
    icon: <ShieldCheck className="h-5 w-5" />,
    title: "Plan Approval Gates",
    description:
      "Optionally pause before execution so you can review, revise, or reject the plan — human-in-the-loop when you need it.",
  },
];

export function Features() {
  return (
    <section id="features" className="relative py-24 md:py-32 dot-grid">
      <div className="mx-auto max-w-6xl px-6">
        <div className="fade-in-up mb-14 text-center">
          <p className="mb-3 font-mono text-xs font-medium uppercase tracking-widest text-accent">
            Features
          </p>
          <h2 className="font-mono text-3xl font-bold tracking-tight md:text-4xl">
            Everything You Need in One Place
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-muted leading-relaxed">
            Stop switching between terminals. EA Code gives you a unified
            interface to configure, execute, and monitor AI-driven development
            workflows.
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
