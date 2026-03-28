import type { ReactNode } from "react";
import { AlertCircle, X } from "lucide-react";
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
        <AlertCircle size={18} strokeWidth={2} className="mt-0.5 shrink-0 text-danger" />
        <div className="flex-1 space-y-1">
          <p className="text-sm font-medium text-fg">Missing prerequisites</p>
          {missing.map((item) => (
            <p key={item.label} className="text-xs text-fg-muted">
              {item.label}{" "}
              <a
                href={item.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-fg-secondary underline underline-offset-2 hover:text-white"
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
          <X size={14} strokeWidth={2} />
        </button>
      </div>
    </div>
  );
}
