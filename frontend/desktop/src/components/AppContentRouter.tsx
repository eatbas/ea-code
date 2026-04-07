import type { ReactNode } from "react";
import type { ActiveView, ConversationDetail, WorkspaceInfo, ProjectEntry, AgentSelection } from "../types";
import type { PendingImage } from "../hooks/useImageAttachments";
import type { PipelineMode } from "./ConversationView/ConversationComposer";
import type { Dispatch, SetStateAction } from "react";
import { IdleView } from "./IdleView";
import { AgentsSettingsRoute } from "./AgentsSettingsView/Route";
import { CliSetupRoute } from "./CliSetupView/Route";
import { GeneralSettingsRoute } from "./GeneralSettingsView/Route";
import { NotificationsSettingsRoute } from "./NotificationsSettingsView/Route";
import { ProjectsSettingsRoute } from "./ProjectsSettingsView/Route";
import { ArchivedThreadsSettingsRoute } from "./ArchivedThreadsSettingsView/Route";
import { ConversationView } from "./ConversationView";

interface AppContentRouterProps {
  activeView: ActiveView;
  workspace: WorkspaceInfo | null;
  sidecarReady: boolean | null;
  viewResetToken: number;
  activeConversation: ConversationDetail | null;
  onSetActiveConversation: Dispatch<SetStateAction<ConversationDetail | null>>;
  activeDraft: string;
  activePromptDraft: string;
  activePipelineMode: PipelineMode;
  onPipelineModeChange: (mode: PipelineMode) => void;
  sendingConversation: boolean;
  stoppingConversation: boolean;
  onPromptDraftChange: (prompt: string) => void;
  onSendConversationPrompt: (prompt: string, agent: AgentSelection, pendingImages?: PendingImage[]) => Promise<void>;
  onStopConversation: () => Promise<void>;
  projects: ProjectEntry[];
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
  onOpenProjectFolder: (path: string) => Promise<void>;
  onOpenInVsCode: (path: string) => Promise<void>;
  allProjects: ProjectEntry[];
  onRemoveProject: (projectPath: string) => void;
  onUnarchiveConversation: (projectPath: string, conversationId: string) => void;
}

/** Routes the main content panel between home and settings views. */
export function AppContentRouter({
  activeView,
  workspace,
  sidecarReady,
  viewResetToken,
  activeConversation,
  onSetActiveConversation,
  activeDraft,
  activePromptDraft,
  activePipelineMode,
  onPipelineModeChange,
  sendingConversation,
  stoppingConversation,
  onPromptDraftChange,
  onSendConversationPrompt,
  onStopConversation,
  projects,
  onSelectProject,
  onAddProject,
  onOpenProjectFolder,
  onOpenInVsCode,
  allProjects,
  onRemoveProject,
  onUnarchiveConversation,
}: AppContentRouterProps): ReactNode {
  if (activeView === "general") {
    return <GeneralSettingsRoute />;
  }

  if (activeView === "notifications") {
    return <NotificationsSettingsRoute />;
  }

  if (activeView === "agents") {
    return <AgentsSettingsRoute />;
  }

  if (activeView === "cli-setup") {
    return <CliSetupRoute />;
  }

  if (activeView === "projects") {
    return (
      <ProjectsSettingsRoute
        projects={allProjects}
        onAddProject={onAddProject}
        onRemoveProject={onRemoveProject}
      />
    );
  }

  if (activeView === "archived-threads") {
    return <ArchivedThreadsSettingsRoute onUnarchiveConversation={onUnarchiveConversation} />;
  }

  if (workspace) {
    return (
      <ConversationView
        workspace={workspace}
        sidecarReady={sidecarReady}
        viewResetToken={viewResetToken}
        activeConversation={activeConversation}
        onSetActiveConversation={onSetActiveConversation}
        activeDraft={activeDraft}
        activePromptDraft={activePromptDraft}
        pipelineMode={activePipelineMode}
        onPipelineModeChange={onPipelineModeChange}
        sending={sendingConversation}
        stopping={stoppingConversation}
        onOpenProjectFolder={onOpenProjectFolder}
        onOpenInVsCode={onOpenInVsCode}
        onPromptDraftChange={onPromptDraftChange}
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
