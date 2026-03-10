import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useToast } from "./Toast";

interface WorkspaceFooterProps {
  path: string;
}

export function WorkspaceFooter({ path }: WorkspaceFooterProps): ReactNode {
  const toast = useToast();

  return (
    <div className="flex w-full items-center justify-between px-1 text-xs text-[#9898b0]">
      <span className="truncate" title={path}>{path}</span>
      <button
        onClick={() => {
          void invoke("open_in_vscode", { path }).catch(() => {
            toast.error("Failed to open VS Code.");
          });
        }}
        className="ml-4 flex shrink-0 items-center gap-1.5 rounded px-2 py-0.5 text-[#9898b0] hover:bg-[#24243a] hover:text-[#e4e4ed] transition-colors"
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
  );
}
