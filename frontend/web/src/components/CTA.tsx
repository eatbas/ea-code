import { Download, Github } from "lucide-react";
import { useReleaseInfo } from "../hooks/useReleaseInfo";

export function CTA() {
  const release = useReleaseInfo();
  const windowsUrl = release?.assets?.windows?.download_url;

  return (
    <section id="download" className="relative py-24 md:py-32">
      <div className="mx-auto max-w-3xl px-6 text-center">
        <div className="fade-in-up rounded-3xl border border-border bg-surface-elevated p-10 md:p-16 shadow-2xl shadow-black/30">
          <h2 className="font-mono text-3xl font-bold tracking-tight md:text-4xl">
            Ready to Orchestrate?
          </h2>
          <p className="mx-auto mt-4 max-w-md text-muted leading-relaxed">
            Download EA Code and turn the AI CLIs on your machine into a
            coordinated, self-improving development team.
          </p>
          <div className="mt-8 flex flex-wrap items-center justify-center gap-4">
            {windowsUrl && (
              <a
                href={windowsUrl}
                className="inline-flex items-center gap-2 rounded-xl bg-accent px-6 py-3 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer"
              >
                <Download className="h-4 w-4" />
                Download {release ? `v${release.version}` : "Latest"}
              </a>
            )}
            <a
              href="https://github.com/eatbas/ea-code"
              className="inline-flex items-center gap-2 rounded-xl border border-border px-6 py-3 text-sm font-medium text-muted transition-colors duration-200 hover:border-muted hover:text-white cursor-pointer"
            >
              <Github className="h-4 w-4" />
              View on GitHub
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
