import type { ReactNode } from "react";

interface WorkspaceFooterProps {
  path: string;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onError: () => void;
}

export function WorkspaceFooter({
  path,
  onOpenProjectFolder,
  onOpenInVsCode,
  onError,
}: WorkspaceFooterProps): ReactNode {
  function handleAction(action: (targetPath: string) => Promise<void>): void {
    void action(path).catch(() => {
      onError();
    });
  }

  return (
    <div className="flex w-full flex-col gap-1.5 text-xs text-fg-muted sm:flex-row sm:items-center sm:justify-between">
      <button
        type="button"
        onClick={() => handleAction(onOpenProjectFolder)}
        className="flex min-w-0 items-center gap-2 rounded px-2 py-0.5 text-left transition-colors hover:bg-elevated hover:text-fg"
        title="Open project folder"
      >
        <svg className="h-4 w-4 shrink-0" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M3 7a2 2 0 0 1 2-2h5l2 2h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        </svg>
        <span className="truncate" title={path}>{path}</span>
      </button>

      <div className="flex shrink-0 items-center justify-end gap-2">
        <button
          type="button"
          onClick={() => handleAction(onOpenInVsCode)}
          className="flex items-center gap-1.5 rounded px-2 py-0.5 text-fg-muted transition-colors hover:bg-elevated hover:text-fg"
          title="Open in VS Code"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M16 3l5 3v12l-5 3L2 12l5-3" />
            <path d="M16 3L7 12l9 9" />
            <path d="M16 3v18" />
          </svg>
          Open in VS Code
        </button>
      </div>
    </div>
  );
}
