import type { ReactNode } from "react";
import type {
  ActiveView,
  ApiCliVersionInfo,
  ApiHealth,
  AppSettings,
  CliHealth,
  ProviderInfo,
  WorkspaceInfo,
  ProjectEntry,
} from "../types";
import { IdleView } from "./IdleView";
import { CliSetupView } from "./CliSetupView";
import { McpView } from "./McpView";

interface AppContentRouterProps {
  activeView: ActiveView;
  workspace: WorkspaceInfo | null;
  projects: ProjectEntry[];
  providers: ProviderInfo[];
  providersLoading: boolean;
  apiVersions: ApiCliVersionInfo[];
  apiVersionsLoading: boolean;
  apiVersionsUpdating: string | null;
  apiHealth: ApiHealth | null;
  cliHealth: CliHealth | null;
  settings: AppSettings | null;
  onSaveSettings: (settings: AppSettings) => void | Promise<void>;
  onFetchApiVersions: () => void;
  onRefreshProviders: () => void;
  onUpdateApiCli: (provider: string) => Promise<void>;
  onSelectProject: (projectPath: string) => Promise<void>;
  onAddProject: () => Promise<void>;
}

/** Routes the main content panel between home and settings views. */
export function AppContentRouter({
  activeView,
  workspace,
  projects,
  providers,
  apiVersions,
  apiVersionsLoading,
  apiVersionsUpdating,
  apiHealth,
  cliHealth,
  settings,
  onSaveSettings,
  onFetchApiVersions,
  onRefreshProviders,
  onUpdateApiCli,
  onSelectProject,
  onAddProject,
}: AppContentRouterProps): ReactNode {
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

  if (activeView === "mcp") return <McpView cliHealth={cliHealth} />;

  return (
    <IdleView
      workspace={workspace}
      workspacePath={workspace?.path}
      projects={projects}
      onSelectProject={onSelectProject}
      onAddProject={onAddProject}
    />
  );
}
