import type { ReactNode } from "react";
import type { WorkspaceInfo } from "../types";

interface HeaderProps {
  workspace: WorkspaceInfo | null;
  onOpenSettings: () => void;
  onBackToHome?: () => void;
  showBackButton: boolean;
}

/** Minimal top navigation bar for the running/completed pipeline view. */
export function Header({ workspace, onOpenSettings, onBackToHome, showBackButton }: HeaderProps): ReactNode {
  return (
    <header className="bg-[#151516] border-b border-[#313134] px-4 py-3 flex items-center justify-between">
      <div className="flex items-center gap-2">
        {showBackButton && onBackToHome && (
          <button
            onClick={onBackToHome}
            className="rounded p-1.5 text-[#8b8b93] hover:bg-[#202022] hover:text-[#f5f5f5] transition-colors"
            title="Back to home"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <line x1="19" y1="12" x2="5" y2="12" />
              <polyline points="12 19 5 12 12 5" />
            </svg>
          </button>
        )}
        <div className="flex items-center gap-2">
          <img src="/logo.png" alt="EA Code logo" className="h-12 w-12" />
          <h1 className="text-lg font-bold text-[#f5f5f5]">EA Code</h1>
        </div>
      </div>

      <span className="font-mono text-sm text-[#8b8b93] truncate max-w-[50%] px-4">
        {workspace ? workspace.path : "No workspace selected"}
      </span>

      <button
        onClick={onOpenSettings}
        className="p-2 rounded hover:bg-[#202022] transition-colors text-[#8b8b93] hover:text-[#f5f5f5]"
        title="Settings"
      >
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
    </header>
  );
}
