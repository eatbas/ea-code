import { useReleaseInfo } from "../hooks/useReleaseInfo";

function GitHubIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0 1 12 6.844a9.59 9.59 0 0 1 2.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0 0 22 12.017C22 6.484 17.522 2 12 2z" />
    </svg>
  );
}

const GITHUB_RELEASES = "https://github.com/eatbas/maestro/releases/latest";

function WindowsIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M3 12V6.5l8-1.1V12H3zm0 .5h8v6.6l-8-1.1V12.5zM12 5.3l9-1.3v8h-9V5.3zm0 7.2h9v8l-9-1.3v-6.7z" />
    </svg>
  );
}

function AppleIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className}>
      <path d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z" />
    </svg>
  );
}

export function CTA() {
  const { release } = useReleaseInfo();
  const windowsUrl = release?.assets?.windows?.download_url;
  const macosUrl = release?.assets?.macos?.download_url;
  const version = release ? `v${release.version}` : "";

  return (
    <section id="download" className="relative py-24 md:py-32">
      <div className="mx-auto max-w-3xl px-6 text-center">
        <div className="fade-in-up rounded-3xl border border-border bg-surface-elevated p-10 md:p-16 shadow-2xl shadow-black/30">
          <h2 className="font-mono text-3xl font-bold tracking-tight md:text-4xl">
            Stop Switching Tabs.
            <br />
            <span className="text-accent">Start Orchestrating.</span>
          </h2>
          <p className="mx-auto mt-4 max-w-md text-muted leading-relaxed">
            Download Maestro and turn the AI CLIs you already pay for
            into a coordinated, self-improving development team.
          </p>
          {version && (
            <p className="mt-2 font-mono text-xs text-accent">{version}</p>
          )}
          <div className="mt-8 flex flex-wrap items-center justify-center gap-4">
            <a
              href={windowsUrl ?? GITHUB_RELEASES}
              className="inline-flex items-center gap-2 rounded-xl bg-accent px-6 py-3 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer"
            >
              <WindowsIcon className="h-4 w-4" />
              Download for Win
            </a>
            <a
              href={macosUrl ?? GITHUB_RELEASES}
              className="inline-flex items-center gap-2 rounded-xl bg-accent px-6 py-3 text-sm font-semibold text-surface transition-colors duration-200 hover:bg-accent-hover cursor-pointer"
            >
              <AppleIcon className="h-4 w-4" />
              Download for Mac
            </a>
            <a
              href="https://github.com/eatbas/maestro"
              className="inline-flex items-center gap-2 rounded-xl border border-border px-6 py-3 text-sm font-medium text-muted transition-colors duration-200 hover:border-muted hover:text-white cursor-pointer"
            >
              <GitHubIcon className="h-4 w-4" />
              View on GitHub
            </a>
          </div>
        </div>
      </div>
    </section>
  );
}
