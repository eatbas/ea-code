import type { ReactNode } from "react";
import type {
  ActiveView,
  ApiCliVersionInfo,
  ApiHealth,
  AppSettings,
  CliHealth,
  CreateSkillPayload,
  PipelineRun,
  ProviderInfo,
  RunOptions,
  SessionDetail,
  Skill,
  UpdateSkillPayload,
  WorkspaceInfo,
  ProjectEntry,
} from "../types";
import { isRunInProgress, isRunTerminalState } from "../utils/statusHelpers";
import { IdleView } from "./IdleView";
import { ChatView } from "./ChatView";
import { SessionDetailView } from "./SessionDetailView";
import { AgentsView } from "./AgentsView";
import { CliSetupView } from "./CliSetupView";
import { SkillsView } from "./SkillsView";
import { McpView } from "./McpView";

interface AppContentRouterProps {
  activeView: ActiveView;
  activeSessionId: string | undefined;
  chatDismissed: boolean;
  run: PipelineRun | null;
  stageLogs: Record<string, string[]>;
  artifacts: Record<string, string>;
  workspace: WorkspaceInfo | null;
  projects: ProjectEntry[];
  sessionDetail: SessionDetail | null;
  sessionDetailLoading: boolean;
  sessionLoadingMore: boolean;
  providers: ProviderInfo[];
  providersLoading: boolean;
  apiVersions: ApiCliVersionInfo[];
  apiVersionsLoading: boolean;
  apiVersionsUpdating: string | null;
  apiHealth: ApiHealth | null;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  skills: Skill[];
  skillsLoading: boolean;
  onSaveSettings: (settings: AppSettings) => void | Promise<void>;
  onFetchApiVersions: () => void;
  onRefreshProviders: () => void;
  onUpdateApiCli: (provider: string) => Promise<void>;
  onCreateSkill: (payload: CreateSkillPayload) => Promise<void>;
  onUpdateSkill: (payload: UpdateSkillPayload) => Promise<void>;
  onDeleteSkill: (id: string) => Promise<void>;
  onMissingAgentSetup: () => void;
  onPausePipeline: (runId?: string) => Promise<void>;
  onResumePipeline: (runId?: string) => Promise<void>;
  onCancelPipeline: (runId?: string) => Promise<void>;
  onRun: (options: RunOptions, sessionIdOverride?: string) => Promise<void>;
  onContinueRun: (options: RunOptions, sessionId?: string) => void;
  onBackFromChat: () => void;
  onBackFromSession: () => void;
  onLoadMoreRuns: () => Promise<void>;
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
}

/** Routes the main content panel between home, chat, session detail, and settings views. */
export function AppContentRouter({
  activeView,
  activeSessionId,
  chatDismissed,
  run,
  stageLogs,
  artifacts,
  workspace,
  projects,
  sessionDetail,
  sessionDetailLoading,
  sessionLoadingMore,
  providers,
  providersLoading,
  apiVersions,
  apiVersionsLoading,
  apiVersionsUpdating,
  apiHealth,
  cliHealth,
  settings,
  skills,
  skillsLoading,
  onSaveSettings,
  onFetchApiVersions,
  onRefreshProviders,
  onUpdateApiCli,
  onCreateSkill,
  onUpdateSkill,
  onDeleteSkill,
  onMissingAgentSetup,
  onPausePipeline,
  onResumePipeline,
  onCancelPipeline,
  onRun,
  onContinueRun,
  onBackFromChat,
  onBackFromSession,
  onLoadMoreRuns,
  onSelectProject,
  onAddProject,
}: AppContentRouterProps): ReactNode {
  const pipelineActive = isRunInProgress(run);
  const pipelineTerminal = isRunTerminalState(run);
  const isHomeRootView = activeView === "home" && !activeSessionId;
  const showChat = isHomeRootView && (pipelineActive || pipelineTerminal) && !chatDismissed;

  if (run && showChat) {
    return (
      <ChatView
        run={run}
        stageLogs={stageLogs}
        artifacts={artifacts}
        providers={providers}
        settings={settings}
        onMissingAgentSetup={onMissingAgentSetup}
        onPause={() => { void onPausePipeline(); }}
        onResume={() => { void onResumePipeline(); }}
        onCancel={() => { void onCancelPipeline(); }}
        onBackToHome={onBackFromChat}
        onContinue={(options) => {
          onContinueRun(options, run.sessionId);
        }}
      />
    );
  }

  if (activeView === "agents" && settings) {
    return (
      <AgentsView
        settings={settings}
        onSave={onSaveSettings}
        providers={providers}
        providersLoading={providersLoading}
      />
    );
  }

  if (activeView === "cli-setup" && settings) {
    return (
      <CliSetupView
        settings={settings}
        apiHealth={apiHealth}
        providers={providers}
        apiVersions={apiVersions}
        versionsLoading={apiVersionsLoading}
        updating={apiVersionsUpdating}
        onFetchVersions={onFetchApiVersions}
        onRefreshProviders={onRefreshProviders}
        onUpdateCli={onUpdateApiCli}
        onSave={onSaveSettings}
      />
    );
  }

  if (activeView === "skills") {
    return (
      <SkillsView
        skills={skills}
        loading={skillsLoading}
        onCreate={onCreateSkill}
        onUpdate={onUpdateSkill}
        onDelete={onDeleteSkill}
      />
    );
  }

  if (activeView === "mcp") return <McpView cliHealth={cliHealth} />;

  if (activeSessionId) {
    return (
      <SessionDetailView
        sessionDetail={sessionDetail}
        loading={sessionDetailLoading}
        stageLogs={stageLogs}
        activeRunId={run?.id}
        providers={providers}
        settings={settings}
        onMissingAgentSetup={onMissingAgentSetup}
        onRun={onRun}
        onPauseRun={(runId) => { void onPausePipeline(runId); }}
        onResumeRun={(runId) => { void onResumePipeline(runId); }}
        onCancelRun={(runId) => { void onCancelPipeline(runId); }}
        onLoadMore={() => { void onLoadMoreRuns(); }}
        loadingMore={sessionLoadingMore}
        onBackToHome={onBackFromSession}
      />
    );
  }

  return (
    <IdleView
      workspace={workspace}
      workspacePath={workspace?.path}
      projects={projects}
      providers={providers}
      settings={settings}
      onMissingAgentSetup={onMissingAgentSetup}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
      onRun={(options) => { void onRun(options); }}
    />
  );
}
