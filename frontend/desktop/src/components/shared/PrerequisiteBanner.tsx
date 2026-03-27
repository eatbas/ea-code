import type { ReactNode } from "react";
import type { PrerequisiteStatus } from "../../types";

interface PrerequisiteBannerProps {
  status: PrerequisiteStatus;
  onDismiss: () => void;
}

interface MissingItem {
  label: string;
  url: string;
  linkText: string;
}

/** Banner shown at the top of the app when system prerequisites are missing. */
export function PrerequisiteBanner({ status, onDismiss }: PrerequisiteBannerProps): ReactNode {
  const missing: MissingItem[] = [];

  if (!status.pythonAvailable) {
    missing.push({
      label: "Python 3.12+ is required but was not found.",
      url: "https://www.google.com/search?q=python+installation+guide",
      linkText: "Install Python",
    });
  }

  if (!status.gitBashAvailable) {
    missing.push({
      label: "Git Bash is required on Windows to run agents.",
      url: "https://www.google.com/search?q=git+bash+installation+windows",
      linkText: "Install Git for Windows",
    });
  }

  if (missing.length === 0) return null;

  return (
    <div className="fixed top-0 right-0 left-0 z-50 border-b border-danger/30 bg-panel/95 px-4 py-3 shadow-lg backdrop-blur-sm">
      <div className="mx-auto flex max-w-3xl items-start gap-3">
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          stroke="#ef4444"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className="mt-0.5 shrink-0"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <div className="flex-1 space-y-1">
          <p className="text-sm font-medium text-fg">Missing prerequisites</p>
          {missing.map((item) => (
            <p key={item.label} className="text-xs text-fg-muted">
              {item.label}{" "}
              <a
                href={item.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-[#d4d4d8] underline underline-offset-2 hover:text-white"
              >
                {item.linkText}
              </a>
            </p>
          ))}
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="shrink-0 rounded p-1 text-fg-faint transition-colors hover:bg-elevated hover:text-fg"
          title="Dismiss"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>
    </div>
  );
}
