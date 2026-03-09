import { ArrowDown } from "lucide-react";

const TERMINAL_LINES = [
  { prefix: "$", text: "ea-code run --prompt 'Add dark mode support'" },
  { prefix: "1", text: "Prompt Enhance .......... done", dim: false },
  { prefix: "2", text: "Skill Select ............ matched 2 skills", dim: false },
  { prefix: "3", text: "Plan .................... 4 steps generated", dim: false },
  { prefix: "4", text: "Plan Audit .............. approved", dim: false },
  { prefix: "5", text: "Generate ................ 3 files modified", dim: false },
  { prefix: "6", text: "Review .................. 1 suggestion", dim: false },
  { prefix: "7", text: "Fix ..................... applied", dim: false },
  { prefix: "8", text: "Judge ................... COMPLETE", dim: false },
  { prefix: "9", text: "Executive Summary ....... ready", dim: false },
];

export function Hero() {
  return (
    <section className="relative overflow-hidden pt-32 pb-20 md:pt-40 md:pb-28">
      <div className="hero-glow" />

      <div className="relative mx-auto max-w-6xl px-6">
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          {/* Copy */}
          <div className="fade-in-up text-center lg:text-left">
            <p className="mb-4 inline-block rounded-full border border-accent/30 bg-accent/10 px-4 py-1.5 font-mono text-xs font-medium text-accent">
              v0.2.0 — Now with 5 AI agents
            </p>
            <h1 className="font-mono text-4xl leading-tight font-bold tracking-tight md:text-5xl lg:text-6xl">
              One App.
              <br />
              <span className="text-accent">Every AI CLI.</span>
              <br />
              Total Control.
            </h1>
            <p className="mt-6 max-w-lg text-lg leading-relaxed text-muted md:text-xl mx-auto lg:mx-0">
              Orchestrate Claude, Codex, Gemini, Kimi, and OpenCode CLIs in a
              self-improving dev loop. Track every stage, iteration, and artefact
              from a single desktop app.
            </p>
            <div className="mt-8 flex flex-wrap items-center justify-center gap-4 lg:justify-start">
              <a
                href="#download"
                className="rounded-xl bg-accent px-6 py-3 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer"
              >
                Download for Free
              </a>
              <a
                href="#pipeline"
                className="flex items-center gap-2 rounded-xl border border-border px-6 py-3 text-sm font-medium text-muted transition-colors duration-200 hover:border-muted hover:text-white cursor-pointer"
              >
                See How It Works
                <ArrowDown className="h-4 w-4" />
              </a>
            </div>
          </div>

          {/* Terminal mock */}
          <div className="fade-in-up mx-auto w-full max-w-lg lg:mx-0" style={{ animationDelay: "0.2s" }}>
            <div className="overflow-hidden rounded-2xl border border-border bg-surface-elevated shadow-2xl shadow-black/40">
              {/* Title bar */}
              <div className="flex items-center gap-2 border-b border-border px-4 py-3">
                <span className="h-3 w-3 rounded-full bg-red-500/80" />
                <span className="h-3 w-3 rounded-full bg-yellow-500/80" />
                <span className="h-3 w-3 rounded-full bg-green-500/80" />
                <span className="ml-3 font-mono text-xs text-muted">ea-code — pipeline</span>
              </div>

              {/* Terminal body */}
              <div className="stagger p-4 font-mono text-[13px] leading-relaxed">
                {TERMINAL_LINES.map((line, i) => (
                  <div key={i} className="flex gap-2">
                    <span className={line.prefix === "$" ? "text-accent" : "text-surface-hover"}>
                      {line.prefix === "$" ? "$" : `[${line.prefix}]`}
                    </span>
                    <span className={i === 0 ? "text-white" : line.text.includes("COMPLETE") ? "text-accent font-bold" : "text-muted"}>
                      {line.text}
                    </span>
                  </div>
                ))}
                <div className="mt-1 flex gap-2">
                  <span className="text-accent">$</span>
                  <span className="terminal-cursor" />
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
