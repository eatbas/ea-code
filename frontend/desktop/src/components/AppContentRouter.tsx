import type { ReactNode } from "react";
import type { ActiveView, ConversationDetail, WorkspaceInfo, ProjectEntry, AgentSelection } from "../types";
import { IdleView } from "./IdleView";
import { CliSetupRoute } from "./CliSetupView/Route";
import { SimpleTaskView } from "./SimpleTaskView";

interface AppContentRouterProps {
  activeView: ActiveView;
  workspace: WorkspaceInfo | null;
  activeConversation: ConversationDetail | null;
  activeDraft: string;
  sendingConversation: boolean;
  stoppingConversation: boolean;
  onSendConversationPrompt: (prompt: string, agent: AgentSelection) => Promise<void>;
  onStopConversation: () => Promise<void>;
  projects: ProjectEntry[];
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
}

/** Routes the main content panel between home and settings views. */
export function AppContentRouter({
  activeView,
  workspace,
  activeConversation,
  activeDraft,
  sendingConversation,
  stoppingConversation,
  onSendConversationPrompt,
  onStopConversation,
  projects,
  onSelectProject,
  onAddProject,
  onOpenProjectFolder,
  onOpenInVsCode,
}: AppContentRouterProps): ReactNode {
  if (activeView === "cli-setup") {
    return <CliSetupRoute />;
  }

  if (workspace) {
    return (
      <SimpleTaskView
        workspace={workspace}
        activeConversation={activeConversation}
        activeDraft={activeDraft}
        sending={sendingConversation}
        stopping={stoppingConversation}
        onOpenProjectFolder={onOpenProjectFolder}
        onOpenInVsCode={onOpenInVsCode}
        onSendPrompt={onSendConversationPrompt}
        onStopConversation={onStopConversation}
      />
    );
  }

  return (
    <IdleView
      workspace={workspace}
      projects={projects}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
      onOpenProjectFolder={onOpenProjectFolder}
      onOpenInVsCode={onOpenInVsCode}
    />
  );
}
