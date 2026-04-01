import type { ReactNode } from "react";
import { WorkspaceFooter } from "../shared/WorkspaceFooter";

interface ConversationFooterProps {
  path: string;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  onError: () => void;
}

export function ConversationFooter({
  path,
  onOpenProjectFolder,
  onOpenInVsCode,
  onError,
}: ConversationFooterProps): ReactNode {
  return (
    <div className="px-5 pb-3 pt-0">
      <div className="mx-auto flex w-full max-w-5xl">
        <WorkspaceFooter
          path={path}
          onOpenProjectFolder={onOpenProjectFolder}
          onOpenInVsCode={onOpenInVsCode}
          onError={onError}
        />
      </div>
    </div>
  );
}
