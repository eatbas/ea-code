import type { ReactNode } from "react";
import type {
  ActiveView,
  AllCliVersions,
  AppSettings,
  CliHealth,
  CreateSkillPayload,
  PipelineRun,
  RunOptions,
  SessionDetail,
  Skill,
  UpdateSkillPayload,
  WorkspaceInfo,
  ProjectSummary,
} from "../types";
import { isRunInProgress, isRunTerminalState } from "../utils/statusHelpers";
import { IdleView } from "./IdleView";
import { ChatView } from "./ChatView";
import { SessionDetailView } from "./SessionDetailView";
import { AgentsView } from "./AgentsView";
import { CliSetupView } from "./CliSetupView";
import { SkillsView } from "./SkillsView";
import { McpView } from "./McpView";
import { AppSettingsView } from "./AppSettingsView";

interface AppContentRouterProps {
  activeView: ActiveView;
  activeSessionId: string | undefined;
  run: PipelineRun | null;
  stageLogs: Record<string, string[]>;
  artifacts: Record<string, string>;
  workspace: WorkspaceInfo | null;
  projects: ProjectSummary[];
  sessionDetail: SessionDetail | null;
  sessionDetailLoading: boolean;
  sessionLoadingMore: boolean;
  versions: AllCliVersions | null;
  versionsLoading: boolean;
  versionsUpdating: string | null;
  cliHealth: CliHealth | null;
  cliHealthChecking: boolean;
  settings: AppSettings | null;
  skills: Skill[];
  skillsLoading: boolean;
  onSaveSettings: (settings: AppSettings) => void | Promise<void>;
  onFetchVersions: (settings: AppSettings) => void;
  onUpdateCli: (cliName: string, settings: AppSettings) => Promise<void>;
  onCreateSkill: (payload: CreateSkillPayload) => Promise<void>;
  onUpdateSkill: (payload: UpdateSkillPayload) => Promise<void>;
  onDeleteSkill: (id: string) => Promise<void>;
  onMissingAgentSetup: () => void;
  onPausePipeline: (runId?: string) => Promise<void>;
  onResumePipeline: (runId?: string) => Promise<void>;
  onCancelPipeline: (runId?: string) => Promise<void>;
  onRun: (options: RunOptions, sessionIdOverride?: string) => Promise<void>;
  onContinueRun: (options: RunOptions, sessionId?: string) => void;
  onNewSession: () => void;
  onLoadMoreRuns: () => Promise<void>;
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
}

/** Routes the main content panel between home, chat, session detail, and settings views. */
export function AppContentRouter({
  activeView,
  activeSessionId,
  run,
  stageLogs,
  artifacts,
  workspace,
  projects,
  sessionDetail,
  sessionDetailLoading,
  sessionLoadingMore,
  versions,
  versionsLoading,
  versionsUpdating,
  cliHealth,
  cliHealthChecking,
  settings,
  skills,
  skillsLoading,
  onSaveSettings,
  onFetchVersions,
  onUpdateCli,
  onCreateSkill,
  onUpdateSkill,
  onDeleteSkill,
  onMissingAgentSetup,
  onPausePipeline,
  onResumePipeline,
  onCancelPipeline,
  onRun,
  onContinueRun,
  onNewSession,
  onLoadMoreRuns,
  onSelectProject,
  onAddProject,
}: AppContentRouterProps): ReactNode {
  const pipelineActive = isRunInProgress(run);
  const pipelineTerminal = isRunTerminalState(run);
  const isHomeRootView = activeView === "home" && !activeSessionId;
  const showChat = isHomeRootView && (pipelineActive || pipelineTerminal);

  if (run && showChat) {
    return (
      <ChatView
        run={run}
        stageLogs={stageLogs}
        artifacts={artifacts}
        cliHealth={cliHealth}
        settings={settings}
        onMissingAgentSetup={onMissingAgentSetup}
        onPause={() => { void onPausePipeline(); }}
        onResume={() => { void onResumePipeline(); }}
        onCancel={() => { void onCancelPipeline(); }}
        onNewSession={onNewSession}
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
        cliHealth={cliHealth}
        cliHealthChecking={cliHealthChecking}
      />
    );
  }

  if (activeView === "cli-setup" && settings) {
    return (
      <CliSetupView
        settings={settings}
        versions={versions}
        loading={versionsLoading}
        updating={versionsUpdating}
        onFetchVersions={onFetchVersions}
        onUpdateCli={onUpdateCli}
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
  if (activeView === "app-settings") return <AppSettingsView />;

  if (activeSessionId) {
    return (
      <SessionDetailView
        sessionDetail={sessionDetail}
        loading={sessionDetailLoading}
        stageLogs={stageLogs}
        activeRunId={run?.id}
        cliHealth={cliHealth}
        settings={settings}
        onMissingAgentSetup={onMissingAgentSetup}
        onRun={onRun}
        onPauseRun={(runId) => { void onPausePipeline(runId); }}
        onResumeRun={(runId) => { void onResumePipeline(runId); }}
        onCancelRun={(runId) => { void onCancelPipeline(runId); }}
        onLoadMore={() => { void onLoadMoreRuns(); }}
        loadingMore={sessionLoadingMore}
        onBackToHome={onNewSession}
      />
    );
  }

  return (
    <IdleView
      workspace={workspace}
      workspacePath={workspace?.path}
      projects={projects}
      cliHealth={cliHealth}
      settings={settings}
      onMissingAgentSetup={onMissingAgentSetup}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
      onRun={(options) => { void onRun(options); }}
    />
  );
}
