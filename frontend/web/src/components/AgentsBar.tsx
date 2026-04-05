const AGENTS = [
  { name: "Claude", mono: "claude", colour: "#D97706" },
  { name: "Codex", mono: "codex", colour: "#10B981" },
  { name: "Gemini", mono: "gemini", colour: "#3B82F6" },
  { name: "Kimi", mono: "kimi", colour: "#A855F7" },
  { name: "OpenCode", mono: "opencode", colour: "#F43F5E" },
];

export function AgentsBar() {
  return (
    <section id="agents" className="border-y border-border bg-white py-10">
      <div className="mx-auto max-w-6xl px-6">
        <p className="mb-6 text-center font-mono text-xs font-medium uppercase tracking-widest text-faint">
          You&apos;re already paying for them &mdash; make them work together
        </p>
        <div className="flex flex-wrap items-center justify-center gap-4 md:gap-6">
          {AGENTS.map((a) => (
            <div
              key={a.mono}
              className="flex items-center gap-3 rounded-xl border border-border bg-white px-5 py-3 transition-all duration-200 hover:border-heading/20 hover:shadow-sm cursor-pointer"
            >
              <span
                className="flex h-9 w-9 items-center justify-center rounded-lg font-mono text-xs font-bold"
                style={{ backgroundColor: a.colour + "14", color: a.colour }}
              >
                {a.mono.slice(0, 2).toUpperCase()}
              </span>
              <span className="font-mono text-sm font-semibold text-heading">{a.name}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
